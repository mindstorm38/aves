//! VirtIO driver.
//! 
//! The implementation follow the [`official specification`] of VIRTIO.
//! 
//! [`official specification`]: https://docs.oasis-open.org/virtio/virtio/v1.2/virtio-v1.2.pdf

use core::ptr::{NonNull, addr_of};
use core::cell::{RefCell, Cell};
use core::num::NonZeroUsize;
use core::mem::size_of;

use bitflags::bitflags;

use crate::memory::page::{PAGE_SIZE, alloc_zeroed, alloc};
use crate::{println, print, write_slice, mmio_struct};
use crate::sync::Mutex;

use super::{Driver, BlockDriver};
use super::block::{BlockDevice, BlockIoResult, BlockIoError};


/// Magic string 'virt' in little-endian.
const VIRTIO_MAGIC: u32 = 0x74_72_69_76;

/// Default queue len used for devices (see [`Queue`]).
const VIRTIO_QUEUE_SIZE: u32 = 1 << 7;

/// Sector size for virtio block devices.
const VIRTIO_BLOCK_SECTOR_SIZE: u64 = 512;


/// Use this driver to provide virtio discovery capabilities.
/// The address, stride and number of ports must be know at
/// compile-time.
pub struct VirtioDriver<const ADDR: usize, const STRIDE: usize, const COUNT: usize> {
    /// Exhaustive list of all devices for all ports (connected or not).
    devices: RefCell<[Option<Device>; COUNT]>,
    /// If the block driver is specified, block devices will be initialized.
    block_driver: Option<&'static BlockDriver>,
}

unsafe impl<const ADDR: usize, const STRIDE: usize, const COUNT: usize> Sync for VirtioDriver<ADDR, STRIDE, COUNT> {}

impl<const ADDR: usize, const STRIDE: usize, const COUNT: usize> VirtioDriver<ADDR, STRIDE, COUNT> {
    
    /// Create the virtio driver.
    pub const fn new() -> Self {
        Self {
            devices: RefCell::new([None; COUNT]),
            block_driver: None,
        }
    }

    /// Enable block devices loading by this virtio driver.
    /// Loaded block devices will be registered in the given block driver.
    pub const fn with_block(mut self, block_driver: &'static BlockDriver) -> Self {
        self.block_driver = Some(block_driver);
        self
    }
    
    /// Iterate over connected devices.
    pub fn iter(&self) -> impl Iterator<Item = Device> + '_ {
        let devices = self.devices.borrow();
        (0..COUNT).filter_map(move |idx| devices[idx])
    }

    /// Iterate over connected devices that are of the given type.
    pub fn iter_type(&self, typ: DeviceType) -> impl Iterator<Item = Device> + '_ {
        self.iter().filter(move |dev| dev.typ == typ)
    }

}

impl<const ADDR: usize, const STRIDE: usize, const COUNT: usize> Driver for VirtioDriver<ADDR, STRIDE, COUNT> {

    fn load(&self) {

        println!("== Loading VirtIO");
        
        for idx in 0..COUNT {

            let addr = ADDR + idx * STRIDE;
            print!(" = Probing device #{} at {:08X}: ", idx, addr);

            let dev = MmioDevice(addr as _);

            if dev.magic_value() != VIRTIO_MAGIC {
                println!("Invalid magic");
                continue;
            }

            let typ = match dev.device_id() {
                0 => {
                    println!("Not connected");
                    continue;
                }
                1 => DeviceType::Network,
                2 => DeviceType::Block,
                4 => DeviceType::Entropy,
                16 => DeviceType::Gpu,
                18 => DeviceType::Input,
                device_id => {
                    println!("Unsupported device type {}", device_id);
                    continue;
                }
            };
            
            println!("{:?} (v{})", typ, dev.version());

            let dev = Device {
                idx,
                mmio: dev,
                typ,
            };

            match typ {
                DeviceType::Block => {
                    if let Some(block_driver) = self.block_driver {
                        load_block_device(block_driver, &dev);
                    }
                }
                _ => {}
            }

            self.devices.borrow_mut()[idx] = Some(dev);
            
        }

    }

    fn unload(&self) {
        
    }

}


/// Enumeration of some of the possible device types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
	Network,
	Block,
	Console,
	Entropy,
	Gpu,
	Input,
}


/// A structure describing a probed device, accessible from
/// the [`VirtioDriver`].
#[derive(Debug, Clone, Copy)]
pub struct Device {
    /// Index of the virtio device.
    pub idx: usize,
    /// Memory-mapped I/O registers of the device.
    pub mmio: MmioDevice,
    /// Device type.
    pub typ: DeviceType,
}


/// A generic virtio queue ("virtqueue") to use with devices.
/// Note that the given queue size must be a power-of-two 
/// (section 2.7 of the specification).
/// 
/// *This virtio queue is actually a "split virtqueue".*
/// 
/// **Should be aligned to page boundary.**
#[repr(C)]
pub struct Queue<const SIZE: usize = {VIRTIO_QUEUE_SIZE as usize}> {
    /// Descriptor table.
    pub descriptor: [QueueDescriptor; SIZE],
    /// Available ring.
    pub available: QueueAvailable<SIZE>,
    /// Used ring. TODO: ALIGNMENT
    pub used: QueueUsed<SIZE>,
}

/// Used in descriptor table. 
#[repr(C, align(16))]
pub struct QueueDescriptor {
    /// Physical address.
    pub addr: u64,
    /// Length of the data.
    pub len: u32,
    /// Should be interpreted and written to using [`QueueDescriptorFlag`].
    pub flags: u16,
    /// Only relevant if `flags` contains [`QueueDescriptorFlag::NEXT`].
    pub next: u16,
}

impl QueueDescriptor {

    pub const fn new(addr: u64, len: u32, write: bool) -> Self {
        Self {
            addr,
            len,
            flags: if write { QueueDescriptorFlag::WRITE.bits() } else { 0 },
            next: 0,
        }
    }

}

#[repr(C, align(2))]
pub struct QueueAvailable<const SIZE: usize> {
    /// Should be interpreted and written to using [`QueueAvailableFlag`].
    pub flags: u16,
    pub index: u16,
    pub ring: [u16; SIZE],
    pub event: u16,
}

/// This structure is aligned to PAGE_SIZE (4096) because we are using the legacy interface.
#[repr(C, align(4096))]
pub struct QueueUsed<const SIZE: usize> {
    /// Should be interpreted and written to using [`QueueUsedFlag`].
    pub flags: u16,
    pub index: u16,
    pub ring: [QueueUsedElement; SIZE],
    pub event: u16,
}

#[repr(C)]
pub struct QueueUsedElement {
    /// Index of start of used descriptor chain.
    pub id: u32,
    /// The number of bytes written into the device writable portion of
    /// the buffer described by the descriptor chain.
    pub len: u32,
}


/// Request structure for block device.
#[repr(C)]
pub struct BlockRequest {
    pub header: BlockRequestHeader,
    pub data: *mut u8,
    /// Written by the device, must be interpreted with [`BlockRequestStatus`].
    pub status: u8,
}

#[repr(C)]
pub struct BlockRequestHeader {
    /// Must be interpreted with [`BlockRequestType`].
    pub typ: u32,
    reserved: u32,
    /// Sector number (sector of 512 bytes). Only relevant if type is
    /// either `In` or `Out`.
    pub sector: u64,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockRequestType {
    In          = 0,
    Out         = 1,
    Flush       = 4,
    GetId       = 8,
    GetLifetime = 10,
    Discard     = 11,
    WriteZeros  = 13,
    SecureErase = 14,
}


#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockRequestStatus {
    Ok          = 0,
    IoError     = 1,
    Unsupported = 2,
}


bitflags! {

    pub struct DeviceStatus: u32 {
        /// Indicates that the guest OS has found the device 
        /// and recognized it as a valid virtio device.
        const ACKNOWLEDGE           = 0x01;
        /// Indicates that the guest OS knows how to drive 
        /// the device.
        const DRIVER                = 0x02;
        /// Indicates that the driver is set up and ready 
        /// to drive the device.
        const DRIVER_OK             = 0x04;
        /// Indicates that the driver has acknowledged all 
        /// the features it understands, and feature 
        /// negotiation is complete.
        const FEATURES_OK           = 0x08;
        /// Indicates that the device has experienced an 
        /// error from which it can???t recover.
        const DEVICE_NEEDS_RESET    = 0x40;
        /// Indicates that something went wrong in the guest, 
        /// and it has given up on the device. This could be 
        /// an internal error, or the driver didn???t like the 
        /// device for some reason, or even a fatal error 
        /// during device operation.
        const FAILED                = 0x80;
    }

    pub struct QueueDescriptorFlag: u16 {
        /// This marks a buffer as continuing via the next field.
        const NEXT      = 0x1;
        /// This marks a buffer as device write-only (otherwise device read-only).
        const WRITE     = 0x2;
        /// This means the buffer contains a list of buffer descriptors.
        const INDIRECT  = 0x4;
    }

    pub struct QueueAvailableFlag: u16 {
        const NO_INTERRUPT = 0x1;
    }

    pub struct QueueUsedFlag: u16 {
        const NO_NOTIFY = 0x1;
    }

    pub struct BlockFeature: u32 {
        /// Device supports request barriers. (legacy)
        const BARRIER_LEGACY        = 1 << 0;
        /// Maximum size of any single segment is in `size_max`.
        const SIZE_MAX              = 1 << 1;
        /// Maximum number of segments in a request is in `seg_max`.
        const SEG_MAX               = 1 << 2;
        /// Disk-style geometry specified in `geometry`.
        const GEOMETRY              = 1 << 4;
        /// Device is read-only.
        const READ_ONLY             = 1 << 5;
        /// Block size of disk is in blk_size.
        const BLOCK_SIZE            = 1 << 6;
        /// Device supports scsi packet commands. (legacy)
        const SCSI_LEGACY           = 1 << 7;
        /// Cache flush command support.
        const FLUSH                 = 1 << 9;
        /// Device exports information on optimal I/O alignment.
        const TOPOLOGY              = 1 << 10;
        /// Device can toggle its cache between writeback and writethrough modes.
        const CONFIG_WCE            = 1 << 11;
        /// Device supports multiqueue.
        const MULTIQUEUE            = 1 << 12;
        /// Device can support discard command, maximum discard sectors size in
        /// `max_discard_sectors` and maximum discard segment number in 
        /// `max_discard_seg`.
        const DISCARD               = 1 << 13;
        /// Device can support write zeroes command, maximum write zeroes
        /// sectors size in `max_write_zeroes_sectors` and maximum write 
        /// zeroes segment number in `max_write_zeroes_seg`.
        const WRITE_ZEROES          = 1 << 14;
        /// Device supports providing storage lifetime information.
        const LIFETIME              = 1 << 15;
        /// Device supports secure erase command, maximum erase sectors
        /// count in `max_secure_erase_sectors` and maximum erase segment 
        /// number in `max_secure_erase_seg`.
        const SECURE_ERASE          = 1 << 16;
    }

}

mmio_struct! {

    pub struct MmioDevice {
        [0x00] sub legacy: MmioLegacyDevice,
        [0x00] r magic_value: u32,
        [0x04] r version: u32,
        [0x08] r device_id: u32,
        [0x0C] r vendor_id: u32,
        [0x10] r device_features: u32,
        [0x14] w set_device_features_sel: u32,
        [0x20] w set_driver_features: u32,
        [0x24] w set_driver_features_sel: u32,
        [0x30] w set_queue_sel: u32,
        [0x34] r queue_num_max: u32,
        [0x38] w set_queue_num: u32,
        [0x44] r queue_ready: u32,
        [0x44] w set_queue_ready: u32,
        [0x50] w set_queue_notify: u32,
        [0x60] r interrupt_status: u32,
        [0x64] w set_interrupt_ack: u32,
        [0x70] r status: u32,
        [0x70] w set_status: u32,
        [0x80] w set_queue_desc_low: u32,
        [0x84] w set_queue_desc_high: u32,
        [0x90] w set_queue_driver_low: u32,
        [0x94] w set_queue_driver_high: u32,
        [0xA0] w set_queue_device_low: u32,
        [0xA4] w set_queue_device_high: u32,
        [0xAC] w set_shared_memory_sel: u32,
        [0xB0] r shared_memory_len_low: u32,
        [0xB4] r shared_memory_len_high: u32,
        [0xB8] r shared_memory_base_low: u32,
        [0xBC] r shared_memory_base_high: u32,
        [0xC0] r queue_reset: u32,
        [0xC0] w set_queue_reset: u32,
        [0xFC] r config_generation: u32,
        [0x100] sub config_block: MmioBlockConfig,
    }

    pub struct MmioLegacyDevice {
        [0x00] sub non_legacy: MmioDevice,
        [0x00] r magic_value: u32,
        [0x04] r version: u32,
        [0x08] r device_id: u32,
        [0x0C] r vendor_id: u32,
        [0x10] r host_features: u32,
        [0x14] w set_host_features_sel: u32,
        [0x20] w set_guest_features: u32,
        [0x24] w set_guest_features_sel: u32,
        [0x28] w set_guest_page_size: u32,
        [0x30] w set_queue_sel: u32,
        [0x34] r queue_num_max: u32,
        [0x38] w set_queue_num: u32,
        [0x3C] w set_legacy_queue_align: u32,
        [0x40] r queue_physical_page_number: u32,
        [0x40] w set_queue_physical_page_number: u32,
        [0x50] w set_queue_notify: u32,
        [0x60] r interrupt_status: u32,
        [0x64] w set_interrupt_ack: u32,
        [0x70] r status: u32,
        [0x70] w set_status: u32,
        [0x100] sub config_block: MmioBlockConfig,
    }

    pub struct MmioBlockConfig {
        [0x00] r capacity: u64,
        [0x08] r size_max: u32,
        [0x0C] r seg_max: u32,
        [0x10] sub geometry: MmioBlockGeometry,
        [0x14] r blk_size: u32,
        [0x18] sub topology: MmioBlockTopology,
        [0x20] r writeback: u8,
        [0x22] r num_queues: u16,
        [0x24] r max_discard_sectors: u32,
        [0x28] r max_discard_seg: u32,
        [0x2C] r discard_sector_alignment: u32,
        [0x30] r max_write_zeroes_sectors: u32,
        [0x34] r max_write_zeroes_seg: u32,
        [0x38] r write_zeroes_may_unmap: u8,
        [0x3C] r max_secure_erase_sectors: u32,
        [0x40] r max_secure_erase_seg: u32,
        [0x44] r secure_erase_sector_alignment: u32,
    }

    pub struct MmioBlockGeometry {
        [0x0] r cylinders: u16,
        [0x2] r heads: u8,
        [0x3] r sectors: u8,
    }

    pub struct MmioBlockTopology {
        [0x0] r physical_block_exp: u8,
        [0x1] r alignment_offset: u8,
        [0x2] r min_io_size: u16,
        [0x4] r opt_io_size: u32,
    }

}


/// This structure handles a virtio queue (allocated in pages) 
/// and tracks the index of the last item appended to the queue.
/// 
/// This structure is intentionnaly not thread-safe (Sync), 
/// therefore you must use it through a mutex, do all the
/// transactions (append descriptors and notify the queue) 
/// and then release the lock.
pub struct QueueHandler<const SIZE: usize = {VIRTIO_QUEUE_SIZE as usize}> {
    /// The actual queue pointer.
    queue: NonNull<Queue<SIZE>>,
    /// The index of the last inserted item.
    index: u16,
}

impl<const SIZE: usize> QueueHandler<SIZE> {

    pub const PAGES_COUNT: usize = (size_of::<Queue<SIZE>>() + PAGE_SIZE - 1) / PAGE_SIZE;

    pub fn new() -> Result<Self, ()> {

        // SAFETY: Allocating here is safe because drivers' loading is single
        // threaded and sequential. And the page count cannot be 0.
        let queue: NonNull<Queue<SIZE>> = unsafe {
            // Note: we use zeroed allocation in order to avoid using 
            match alloc_zeroed(NonZeroUsize::new_unchecked(Self::PAGES_COUNT)) {
                Ok(ptr) => ptr.cast(),
                Err(_) => return Err(()),
            }
        };

        Ok(Self {
            queue,
            index: 0,
        })

    }

    #[inline]
    pub fn size(&self) -> u32 {
        SIZE as u32
    }

    #[inline]
    pub fn page_size(&self) -> u32 {
        PAGE_SIZE as u32
    }

    #[inline]
    pub fn page_number(&self) -> u32 {
        (self.queue.addr().get() / PAGE_SIZE) as u32
    }

    pub fn append<'a, 'b: 'a>(&'a mut self, descriptor: QueueDescriptor) -> QueueHandlerNext<'a, 'b, SIZE> {
        
        let index = ((self.index as u32 + 1) % SIZE as u32) as u16;
        self.index = index;

        // Note: the reference here has an unbound lifetime.
        let queue = unsafe { self.queue.as_mut() };
        queue.descriptor[index as usize] = descriptor;

        let queue = &mut queue.descriptor[index as usize];

        // SAFETY: Here we leak two mutable references, both for the
        // handler and the previous descriptor. This is safe because
        // this is not exposed and the caller can't call 'append'
        // if this object is existing.
        QueueHandlerNext {
            index,
            head_index: index,
            handler: self, // 'a
            prev: queue    // 'b
        }

    }

    /// Mark the given descriptor index has available for the device.
    pub fn mark_available(&mut self, head_index: u16) {
        let queue = unsafe { self.queue.as_mut() };
        let index = queue.available.index;
        queue.available.ring[index as usize] = head_index;
        queue.available.index = ((index as u32 + 1) % SIZE as u32) as u16;
    }

}

pub struct QueueHandlerNext<'a, 'b: 'a, const SIZE: usize> {
    /// Index of the previously inserted descriptor.
    index: u16,
    /// Index of the first inserted descriptor in the chain, this is constant over calls
    /// to `next`.
    head_index: u16,
    /// Mutable reference to the queue handler. This mutable reference and its lifetime
    /// prevent the caller from appending to the handler while an instance of this
    /// structure is existing.
    handler: &'a mut QueueHandler<SIZE>,
    /// Mutable reference to the previous descriptor.
    prev: &'b mut QueueDescriptor,
}

impl<'a, 'b: 'a, const SIZE: usize> QueueHandlerNext<'a, 'b, SIZE> {

    pub fn next<'b_: 'a>(mut self, descriptor: QueueDescriptor) -> QueueHandlerNext<'a, 'b_, SIZE> {
        let mut next = self.handler.append(descriptor);
        // The head index should not change over calls to 'next'.
        next.head_index = self.head_index;
        self.prev.flags |= QueueDescriptorFlag::NEXT.bits();
        self.prev.next = self.index;
        next
    }

    /// Return the index of the previously inserted descriptor.
    #[inline]
    pub fn index(&self) -> u16 {
        self.index
    }

    /// Return the index of the previously inserted descriptor.
    #[inline]
    pub fn head_index(&self) -> u16 {
        self.head_index
    }

}


///////////////////////
//// BLOCK DEVICES ////
///////////////////////

/// Data used for block device.
pub struct BlockDeviceData {
    pub mmio: MmioLegacyDevice,
    pub queue: QueueHandler,
}


/// Called to load a block device.
fn load_block_device(block_driver: &BlockDriver, dev: &Device) {

    let dev_version = dev.mmio.version();
    if dev_version != 1 {
        println!("   Version {} is not suppported for block devices", dev_version);
        return;
    }
    
    // We know that it's a v1 interface, so interpret it's structure as-is.
    let mmio = dev.mmio.legacy();
    let config = mmio.config_block();

    // 1. Reset the device.
    mmio.set_status(0);

    // 2. We noticed the device.
    let mut status = DeviceStatus::ACKNOWLEDGE;
    mmio.set_status(status.bits());

    // 3. We known how to drive the device.
    status |= DeviceStatus::DRIVER;
    mmio.set_status(status.bits());

    // 4. Read device features and acknowledge understood features.
    let host_features = mmio.host_features();
    let read_only = host_features & BlockFeature::READ_ONLY.bits() != 0;
    mmio.set_guest_features(host_features);

    // 5. Set features ok flag to signal that we choosed.
    status |= DeviceStatus::FEATURES_OK;
    mmio.set_status(status.bits());

    // 6. Read-read status to ensure that host is okay with our flags.
    if mmio.status() & DeviceStatus::FEATURES_OK.bits() == 0 {
        println!("   Unsupported features");
        return;
    }

    // 7. Configure our device, we will only use the first queue #0.
    // First, we get the maximum size of the queue.
    mmio.set_queue_sel(0);
    if mmio.queue_num_max() < VIRTIO_QUEUE_SIZE {
        println!("   Queue too short");
        return;
    }
    
    let queue = match QueueHandler::new() {
        Ok(queue) => queue,
        Err(()) => {
            println!("   Failed queue allocation");
            return;
        }
    };

    mmio.set_queue_num(queue.size());
    mmio.set_guest_page_size(queue.page_size());
    mmio.set_queue_physical_page_number(queue.page_number());

    // 8. Our driver is operationnal!
    status |= DeviceStatus::DRIVER_OK;
    mmio.set_status(status.bits());

    // Note that capacity is expressed in number of 512-bytes sectors.
    println!("   Capacity of {} bytes", config.capacity() * VIRTIO_BLOCK_SECTOR_SIZE);
    
    // Construct our block device data for registering it to the block driver.
    // We put the device data in a mutex because we need to access it safely
    // accross threads.
    let dev_data = Mutex::new(BlockDeviceData {
        mmio,
        queue,
    });

    fn do_read(data: &Mutex<BlockDeviceData>, dst: &mut [u8], off: u64) -> BlockIoResult<()> {
        do_block_operation(data, dst.as_mut_ptr(), dst.len(), off, false)
    }

    fn do_write(data: &Mutex<BlockDeviceData>, src: &[u8], off: u64) -> BlockIoResult<()> {
        do_block_operation(data, src.as_ptr() as _, src.len(), off, true)
    }

    let mut block_dev = BlockDevice::new(dev_data, do_read, (!read_only).then_some(do_write), VIRTIO_BLOCK_SECTOR_SIZE);
    write_slice!(block_dev.raw_name_mut(), "virtio{:02}", dev.idx).unwrap();
    block_driver.register(block_dev);

}


fn do_block_operation(data: &Mutex<BlockDeviceData>, buf: *mut u8, len: usize, off: u64, write: bool) -> BlockIoResult<()> {

    // Lock for the whole operation.
    let mut data = data.spin_lock();

    // Sectors are 512 bytes 
    let sector = off / VIRTIO_BLOCK_SECTOR_SIZE;

    // Allocate a temporary request structure that take an entire page.
    // FIXME: In the future, improve the allocation strategy.
    let mut block_request: NonNull<BlockRequest> = unsafe {
        alloc(NonZeroUsize::new_unchecked(size_of::<BlockRequest>())).map_err(|_| BlockIoError::Internal)?.cast()
    };

    // SAFETY: We own the only pointer to request, so the following mut ref
    // is legal for the rest of the function.
    let block_request = unsafe { block_request.as_mut() };

    // Fill 
    block_request.header.sector = sector;
    block_request.header.typ = if write { BlockRequestType::Out } else { BlockRequestType::In } as _;
    block_request.header.reserved = 0;
    block_request.data = buf;
    block_request.status = 111;

    let head_index = data.queue
        .append(QueueDescriptor::new(addr_of!(block_request.header).addr() as u64, size_of::<BlockRequestHeader>() as u32, false))
        .next(QueueDescriptor::new(buf.addr() as u64, len as u32, !write)) // TODO: Check !write
        .next(QueueDescriptor::new(addr_of!(block_request.status).addr() as u64, 1, true))
        .head_index();

    data.queue.mark_available(head_index);
    
    // Notify the queue 0 as it is the only one used.
    data.mmio.set_queue_notify(0);

    Ok(())

}
