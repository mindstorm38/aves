//! Management of the Platform-Level Interrupt Controller.

use core::num::NonZeroU8;


const PLIC_PRIORITY: *mut u32           = 0x0C00_0000 as *mut u32;
const PLIC_ENABLE: *mut u32             = 0x0C00_2000 as *mut u32;
const PLIC_THRESHOLD: *mut u32          = 0x0C20_0000 as *mut u32;
const PLIC_CLAIM_AND_COMPLETE: *mut u32 = 0x0C20_0004 as *mut u32;


/// Unable an interrupt given its id (1..=31).
pub unsafe fn enable(id: u8) {
    PLIC_ENABLE.write_volatile(PLIC_ENABLE.read_volatile() | (1 << id));
}

/// Disable an interrupt given its id (1..=31).
pub unsafe fn disable(id: u8) {
    PLIC_ENABLE.write_volatile(PLIC_ENABLE.read_volatile() & !(1 << id));
}

/// Return true if the given interrupt is enabled.
pub unsafe fn is_enabled(id: u8) -> bool {
    PLIC_ENABLE.read_volatile() & (1 << id) != 0
}

/// Set the priority (0..=7) of the given interrupt.
pub unsafe fn set_priority(id: u8, priority: u8) {
    PLIC_PRIORITY.add(id as usize).write_volatile(priority as u32 & 0b111);
}

/// Get the priority (0..=7) of the given interrupt.
pub unsafe fn get_priority(id: u8) -> u8 {
    PLIC_PRIORITY.add(id as usize).read_volatile() as u8 & 0b111
}

/// Set the global threshold (0..=7). 
pub unsafe fn set_threshold(global_threshold: u8) {
    PLIC_THRESHOLD.write_volatile(global_threshold as u32 & 0b111);
}

/// Get the global threshold (0..=7). 
pub unsafe fn get_threshold() -> u8 {
    PLIC_THRESHOLD.read_volatile() as u8 & 0b111
}

/// Claim the next available interrupt.
pub unsafe fn claim() -> Option<NonZeroU8> {
    NonZeroU8::new(PLIC_CLAIM_AND_COMPLETE.read_volatile() as u8)
}

/// Mark the previously claimed interrupt as completed.
pub unsafe fn complete(id: u8) {
    PLIC_CLAIM_AND_COMPLETE.write_volatile(id as u32);
}
