//! Core block device driver.

use core::mem::{MaybeUninit, size_of, transmute};
use core::cell::RefCell;

use super::Driver;


/// Maximum number of block devices.
/// 
/// *This might not be the definitive form, but for now we have a fixed length.*
const BLOCK_DEVICE_MAX_LEN: usize = 256;

/// Maximum length for the block device name.
const BLOCK_DEVICE_NAME_LEN: usize = 16;

/// Allow 64 bytes of custom data for block devices.
const BLOCK_DEVICE_DATA_LEN: usize = 64;


/// This driver must be used by other drivers to register block devices
/// and their callbacks in order to provide a uniformized API to 
/// higher-level storage drivers.
pub struct BlockDriver {
    devices: RefCell<([MaybeUninit<BlockDevice>; BLOCK_DEVICE_MAX_LEN], usize)>,
}

/// The driver is sync between threads, because the devices cell is only
/// modified on startup (single-threaded and sequential).
unsafe impl Sync for BlockDriver {}

impl BlockDriver {

    pub const fn new() -> Self {
        Self {
            devices: RefCell::new((unsafe { MaybeUninit::uninit().assume_init() }, 0)),
        }
    }

    /// Register a new block device.
    pub fn register(&self, dev: BlockDevice) {

        let mut borrow = self.devices.borrow_mut();
        let (devices, len) = &mut *borrow;

        debug_assert_ne!(*len, BLOCK_DEVICE_MAX_LEN, "reached max number of block devices");
        devices[*len] = MaybeUninit::new(dev);
        *len += 1;

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
    name: [u8; BLOCK_DEVICE_NAME_LEN],
    /// Data depending on the block device's type.
    data: [u8; BLOCK_DEVICE_DATA_LEN],
    /// Read operation on this device.
    read: fn(data: *const u8, dst: &mut [u8], off: u64),
    /// Write operation on this device, none if this 
    /// block device is read-only.
    write: Option<fn(data: *const u8, src: &[u8], off: u64)>,
}

impl BlockDevice {

    /// Construct a new block device with a custom data and 
    /// access callbacks.
    /// 
    /// *The given name should not contains nul chars and 
    /// must be ascii.*
    pub fn new<D>(
        data: D, 
        read: fn(data: &D, dst: &mut [u8], off: u64),
        write: Option<fn(data: &D, src: &[u8], off: u64)>
    ) -> Self {
        debug_assert!(size_of::<D>() <= BLOCK_DEVICE_DATA_LEN, "given data is too big to be stored inline in block device (max is {} bytes)", BLOCK_DEVICE_DATA_LEN);
        Self {
            name: [0; BLOCK_DEVICE_NAME_LEN],
            data: [0; BLOCK_DEVICE_DATA_LEN],
            read: unsafe { transmute(read) },
            write: write.map(|write| unsafe { transmute(write) })
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

    pub fn read_only(&self) -> bool {
        self.write.is_none()
    }

    pub fn read(&self, dst: &mut [u8], off: u64) {
        (self.read)(self.data.as_ptr(), dst, off);
    }

    pub fn write(&self, src: &[u8], off: u64) {
        if let Some(write) = self.write {
            write(self.data.as_ptr(), src, off);
        } else {
            panic!("this block device is read-only, you should not write to it");
        }
    }

}
