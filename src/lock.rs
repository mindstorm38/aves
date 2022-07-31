use core::arch::asm;
use core::cell::UnsafeCell;



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

}


pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}
