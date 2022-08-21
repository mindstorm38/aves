//! Driver

pub mod virtio;
pub mod block;

pub use virtio::VirtioDriver;
pub use block::BlockDriver;


/// Definition of a driver and it's callbacks.
pub trait Driver: Sync {
    
    /// Called once when the driver is loaded.
    fn load(&self);

    /// Called once when the driver is unloaded.
    fn unload(&self);

}
