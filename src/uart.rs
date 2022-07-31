use core::fmt::Write;


/// The default UART interface.
pub static mut DEFAULT: Uart = Uart::new(0x1000_0000);


/// Initialize the default UART interface.
#[inline]
pub unsafe fn init() {
    DEFAULT.init();
}

/// Get the character from the default UART interface.
#[inline]
pub unsafe fn get() -> Option<u8> {
    DEFAULT.get()
}

/// Put a character to the default UART interface.
#[inline]
pub unsafe fn put(value: u8) {
    DEFAULT.put(value)
}


/// UART 16550 datasheet: http://caro.su/msx/ocm_de1/16550.pdf
pub struct Uart {
    base_addr: *mut u8
}

unsafe impl Sync for Uart {}

impl Uart {

    pub const fn new(base_addr: usize) -> Self {
        Self { base_addr: base_addr as *mut u8 }
    }

    pub unsafe fn init(&mut self) {

        let ptr = self.base_addr;

        const LCR: u8 = 0b00000011;

        // Set word length to 8 bits
        ptr.add(3).write_volatile(LCR);

        // Enable fifo.
        ptr.add(2).write_volatile(0b00000001);

        // Enable data-ready interrupt.
        ptr.add(1).write_volatile(0b00000001);

        // Set DLAB to 1 and keep word length to 8 bits.
        ptr.add(3).write_volatile(0b10000000 | LCR);

        // Compute DLL and DLM.
        let divisor: u16 = 592;
        let divisor_least = (divisor >> 0) as u8;
        let divisor_most = (divisor >> 8) as u8;
        ptr.add(0).write_volatile(divisor_least);
        ptr.add(1).write_volatile(divisor_most);

        // Reset DLAB to zero.
        ptr.add(3).write_volatile(LCR);

    }

    #[inline]
    pub unsafe fn get(&self) -> Option<u8> {
        let ptr = self.base_addr;
        if ptr.add(5).read_volatile() & 1 == 0 {
            None
        } else {
            Some(ptr.add(0).read_volatile())
        }
    }

    #[inline]
    pub unsafe fn put(&mut self, value: u8) {
        self.base_addr.add(0).write_volatile(value);
    }

}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            unsafe { self.put(c); }
        }
        Ok(())
    }
}
