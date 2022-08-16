//! This module includes the assembly files.
//! 
//! Also redefine the symbols found both in the
//! assembly and in the linker script for use
//! in Rust.

use core::arch::global_asm;

global_asm!(include_str!("asm/boot.asm"));
global_asm!(include_str!("asm/trap.asm"));
global_asm!(include_str!("asm/sym.asm"));
global_asm!(include_str!("asm/proc.asm"));

extern "C" {

    /// Assembly function to abort a hart, 
    /// this is not recoverable as it will
    /// loop indefinitely until reset.
    pub fn asm_abort() -> !;

    /// Assembly function used as trap vector,
    /// this symbol is just defined here for
    /// documentation.
    /// 
    /// *This function should not be called
    /// directly, because it's a trap vector
    /// and the return statement uses `mret`.
    pub fn asm_trap_vector() -> !;

    pub static LD_MEMORY_START: *mut u8;
    pub static LD_MEMORY_END: *mut u8;
    pub static LD_MEMORY_SIZE: usize;
    
    pub static LD_KSTACK_START: *mut u8;
    pub static LD_KSTACK_END: *mut u8;

    pub static LD_HEAP_START: *mut u8;
    pub static LD_HEAP_SIZE: usize;

}
