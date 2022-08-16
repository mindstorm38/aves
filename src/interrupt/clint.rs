//! Management of the Core-Local Interruptor.
//! 
//! This is defined [`here`].
//! 
//! [`here`]: https://sifive.cdn.prismic.io/sifive%2F834354f0-08e6-423c-bf1f-0cb58ef14061_fu540-c000-v1.0.pdf#%5B%7B%22num%22%3A157%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C0%2C630%2C0%5D


/// Memory-mapped registers for setting *Machine Software-Interrupt Pending*
/// for specific harts.
const CLINT_MSIP: *mut u32 = 0x0020_0000 as *mut u32;

/// Memory-mapped registers for setting *mtimecmp* for a specific hart.
const CLINT_MTIMECMP: *mut u64 = 0x0020_4000 as *mut u64;

/// Memory-mapped register that contains the number of cycles counted
/// from the `RTCCLK` input.
const CLINT_MTIME: *mut u64 = 0x0020_BFF8 as *mut u64;


/// Set the MSIP flag for a specific hart through the 
/// memory-mapped register of the given hart.
#[inline]
pub unsafe fn set_msip(hartid: usize) {
    CLINT_MSIP.add(hartid).write_volatile(1);
}

#[inline]
pub unsafe fn get_msip(hartid: usize) -> bool {
    CLINT_MSIP.add(hartid).read_volatile() != 0
}

#[inline]
pub unsafe fn set_mtimecmp(hartid: usize, timecmp: u64) {
    CLINT_MTIMECMP.add(hartid).write_volatile(timecmp);
}

#[inline]
pub unsafe fn get_mtimecmp(hartid: usize) -> u64 {
    CLINT_MTIMECMP.add(hartid).read_volatile()
}

#[inline]
pub unsafe fn set_mtime(time: u64) {
    CLINT_MTIME.write_volatile(time);
}

#[inline]
pub unsafe fn get_mtime() -> u64 {
    CLINT_MTIME.read_volatile()
}
