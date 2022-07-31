//! This module includes the assembly files and 
//! redefine the symbols found in them for use
//! in Rust.

use core::arch::global_asm;

global_asm!(include_str!("asm/boot.asm"));
global_asm!(include_str!("asm/trap.asm"));
global_asm!(include_str!("asm/sym.asm"));

extern "C" {

    pub fn asm_abort() -> !;

    pub static LD_MEMORY_START: *mut u8;
    pub static LD_MEMORY_END: *mut u8;
    pub static LD_MEMORY_SIZE: usize;
    
    pub static LD_KSTACK_START: *mut u8;
    pub static LD_KSTACK_END: *mut u8;

    pub static LD_HEAP_START: *mut u8;
    pub static LD_HEAP_SIZE: usize;

}
