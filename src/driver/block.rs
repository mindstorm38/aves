//! Core block device driver.

use core::mem::{MaybeUninit, transmute};

use crate::util::OpaqueCell;
use crate::sync::Mutex;
use super::Driver;


/// Maximum number of block devices.
/// 
/// *This might not be the definitive form, but for now we have a fixed length.*
pub const BLOCK_DEVICE_COUNT: usize = 256;

/// Maximum length for the block device name.
pub const BLOCK_DEVICE_NAME_SIZE: usize = 16;

/// Allow 64 bytes of custom data for block devices.
pub const BLOCK_DEVICE_DATA_SIZE: usize = 64;


/// This driver must be used by other drivers to register block devices
/// and their callbacks in order to provide a uniformized API to 
/// higher-level storage drivers.
pub struct BlockDriver {
    /// Registered devices.
    devices: Mutex<BlockDevices>,
}

/// Vector of block devices currently registered.
struct BlockDevices {
    devices: [MaybeUninit<BlockDevice>; BLOCK_DEVICE_COUNT],
    len: usize,
}

impl BlockDriver {

    pub const fn new() -> Self {
        Self {
            devices: Mutex::new(BlockDevices {
                devices: unsafe { MaybeUninit::uninit().assume_init() },
                len: 0,
            }),
        }
    }

    /// Register a new block device.
    pub fn register(&self, dev: BlockDevice) {

        let mut borrow = self.devices.spin_lock();

        let len = borrow.len;
        debug_assert_ne!(len, BLOCK_DEVICE_COUNT, "reached max number of block devices");
        borrow.devices[len] = MaybeUninit::new(dev);
        borrow.len = len + 1;

    }

}

impl Driver for BlockDriver {

    fn load(&self) {
        
    }

    fn unload(&self) {
        
    }

}


/// A fixed-size structure stored by [`BlockDriver`] that provides
/// an abstracted API for access block devices.
pub struct BlockDevice {
    /// UTF-8, nul-termined name of the block device.
    name: [u8; BLOCK_DEVICE_NAME_SIZE],
    /// The opaque cell containing the custom data.
    data: OpaqueCell<BLOCK_DEVICE_DATA_SIZE>,
    /// Read operation on this device.
    read: fn(data: *const u8, dst: &mut [u8], off: u64) -> BlockIoResult<()>,
    /// Write operation on this device, none if this 
    /// block device is read-only.
    write: Option<fn(data: *const u8, src: &[u8], off: u64) -> BlockIoResult<()>>,
    /// The sector size of the device.
    sector_size: u64,
}

impl BlockDevice {

    /// Construct a new block device with a custom data and 
    /// access callbacks. The custom data must be synchronizable
    /// between threads because read and writes can happen from
    /// any thread.
    /// 
    /// *The given name should not contains nul chars and 
    /// must be ascii.*
    pub fn new<D: Sync>(
        data: D, 
        read: fn(data: &D, dst: &mut [u8], off: u64) -> BlockIoResult<()>,
        write: Option<fn(data: &D, src: &[u8], off: u64) -> BlockIoResult<()>>,
        sector_size: u64,
    ) -> Self {

        // SAFETY: Here the transmutation is safe because &D as the same
        // layout as *const u8.
        // https://rust-lang.github.io/unsafe-code-guidelines/layout/pointers.html
        Self {
            name: [0; BLOCK_DEVICE_NAME_SIZE],
            data: OpaqueCell::new(data),
            read: unsafe { transmute(read) },
            write: write.map(|write| unsafe { transmute(write) }),
            sector_size
        }

    }

    /// Before registering the block device to the drive, use this to
    /// set the name of the block device.
    pub fn raw_name_mut(&mut self) -> &mut [u8] {
        &mut self.name[..]
    }

    pub fn name(&self) -> &str {
        let len = self.name.iter().position(|b| *b == 0).unwrap_or(self.name.len());
        unsafe { core::str::from_utf8_unchecked(&self.name[..len]) }
    }

    pub fn sector_size(&self) -> u64 {
        self.sector_size
    }

    pub fn read_only(&self) -> bool {
        self.write.is_none()
    }

    pub fn read(&self, dst: &mut [u8], off: u64) -> BlockIoResult<()> {
        (self.read)(self.data.as_ptr(), dst, off)
    }

    pub fn write(&self, src: &[u8], off: u64) -> BlockIoResult<()> {
        if let Some(write) = self.write {
            write(self.data.as_ptr(), src, off)
        } else {
            Err(BlockIoError::ReadOnly)
        }
    }

}


pub type BlockIoResult<T> = Result<T, BlockIoError>;


#[derive(Debug)]
pub enum BlockIoError {
    /// The block device is read-only.
    ReadOnly,
    /// The given offset is not aligned to a sector of the block device.
    UnalignedOffset,
    /// Internal error of the backend of the block device.
    Internal,
}
