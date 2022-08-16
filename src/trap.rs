//! Machine trap management, this module is coupled with
//! `trap.asm` as it defines the trap frame and the data
//! structure should be exactly following in the assembly.
//! 
//! CLINT: Core-Local Interruptor
//! PLIC:  Platform-Level Interrupt Controller

use core::mem::MaybeUninit;

use crate::cpu::mie::MieFlags;
use crate::cpu;


const MAX_TRAPS_COUNT: usize = 256;


/// Structure used to temporarily save registers that 
/// might be used in the trap handler.
#[repr(C)]
pub struct TrapFrame {
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub traps_count: usize,
    pub traps: [MaybeUninit<Trap>; MAX_TRAPS_COUNT],
}

/// Represent a trap
#[repr(C)]
pub struct Trap {
    pub mcause: usize,
    pub mtval: usize,
    pub mepc: usize,
}

impl TrapFrame {
    
    pub const fn new() -> Self {
        Self {
            t0: 0,
            t1: 0,
            t2: 0,
            traps_count: 0,
            traps: unsafe { MaybeUninit::uninit().assume_init() }
        }
    }

}

impl Trap {

    #[inline(always)]
    pub fn interrupt(&self) -> bool {
        self.mcause & (1 << (usize::BITS - 1)) != 0
    }

    #[inline(always)]
    pub fn code(&self) -> usize {
        self.mcause & ((1 << (usize::BITS - 1)) - 1)
    }

    #[inline(always)]
    pub fn val(&self) -> usize {
        self.mtval
    }

    #[inline(always)]
    pub fn pc(&self) -> usize {
        self.mepc
    }

}


/// The frame used to save the context when hart #0 is interrupted.
static mut HART_ZERO_FRAME: TrapFrame = TrapFrame::new();


/// Initialize trap (for hart #0 only).
pub unsafe fn init_hart_zero() {
    cpu::mie::set(MieFlags::MEIE);
    cpu::mscratch::set((&mut HART_ZERO_FRAME as *mut TrapFrame).addr());
}


