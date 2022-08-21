//! Synchronization primitives for the kernel-side.

use core::arch::asm;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};


#[repr(u32)]
enum MutexState {
    Released = 0,
    Acquired = 1,
}

const STATE_RELEASED: u32 = MutexState::Released as u32;
const STATE_ACQUIRED: u32 = MutexState::Acquired as u32;


#[repr(C)]
pub struct Mutex<T: ?Sized> {
    state: MutexState,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized> Sync for Mutex<T> {}

impl<T> Mutex<T> {

    pub const fn new(data: T) -> Self {
        Self { 
            state: MutexState::Released, 
            data: UnsafeCell::new(data)
        }
    }

}

impl<T: ?Sized> Mutex<T> {

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        let state: u32;
        unsafe {
            asm!("amoswap.w.aq {0}, {1}, ({2})", out(reg) state, in(reg) STATE_ACQUIRED, in(reg) &self.state);
        }
        match state {
            STATE_RELEASED => Some(MutexGuard { mutex: self }),
            _ => None
        }
    }

    // pub fn lock(&self) -> MutexGuard<'_, T> {
    //     loop {
    //         if let Some(guard) = self.try_lock() {
    //             return guard;
    //         }
    //         unsafe {
    //             // To avoid looping too fast, we wait for interrupt,
    //             // and we will send a user interrupt on unlock.
    //             asm!("wfi");
    //         }
    //     }
    // }

    /// To use inside interrupt context.
    pub fn spin_lock(&self) -> MutexGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }
        }
    }

    /// Internal function used by guard to unlock the lock.
    fn unlock(&self) {
        unsafe {
			asm!(
                "amoswap.w.rl zero, zero, ({0})",
                // TODO : Trigger software interrupt
                in(reg) &self.state
            );
		}
    }

}


pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() } 
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() } 
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}
