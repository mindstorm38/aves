//! Driver

pub mod virtio;
pub mod block;

pub use virtio::VirtioDriver;
pub use block::BlockDriver;


/// Definition of a driver and it's callbacks.
pub trait Driver {
    
    fn load(&self);

    fn unload(&self);

}


const DRIVERS_MAX_COUNT: usize = 512;
static mut DRIVERS: [Option<&'static dyn Driver>; DRIVERS_MAX_COUNT] = [None; DRIVERS_MAX_COUNT];
static mut DRIVERS_COUNT: usize = 0;


/// Register a driver at startup.
/// 
/// *This function is unsafe because it should be called only
/// on kernel's startup.*
pub unsafe fn register(driver: &'static dyn Driver) {
    debug_assert_ne!(DRIVERS_COUNT, DRIVERS_MAX_COUNT, "no more space to register driver");
    DRIVERS[DRIVERS_COUNT] = Some(driver);
    DRIVERS_COUNT += 1;
}


/// Iterate over all drivers.
pub fn iter() -> impl Iterator<Item = &'static dyn Driver> {
    // SAFETY: This should be safe because the register function
    // is called once on startup, and the following accesses to
    // drivers static variables never mutates.
    unsafe { (0..DRIVERS_COUNT).map(|i| DRIVERS[i].unwrap_unchecked()) }
}
