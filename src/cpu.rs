//! Access to low-level Control and Status registers.

pub mod mhardid {
    /// Get the hardware thread (hart) identifier executing this function.
    #[inline(always)]
    pub fn get() -> usize {
        let hard_id;
        unsafe { core::arch::asm!("csrr {0}, mhartid", out(reg) hard_id); }
        hard_id
    }
}

pub mod misa {

    bitflags::bitflags! {

        /// Extensions supported by the ISA.
        pub struct Extensions: u32 {
            /// Atomic extension.
            const ATOMIC        = 1 << 0;
            /// Tentatively reserved for Bit-Manipulation extension.
            const BIT_MANIP     = 1 << 1;
            /// Compressed extension.
            const COMPRESSED    = 1 << 2;
            /// Double-precision floating-point extension.
            const FLOAT64       = 1 << 3;
            /// RV32E base ISA.
            const EMBEDDED      = 1 << 4;
            /// Single-precision floating-point extension.
            const FLOAT32       = 1 << 5;
            /// Hypervisor extension.
            const HYPERVISOR    = 1 << 7;
            /// Base integer instruction set.
            const INT           = 1 << 8;
            /// Integer Multiply/Divide extension.
            const MATH          = 1 << 12;
            /// Tentatively reserved for Packed-SIMD extension.
            const PACKED_SIMD   = 1 << 15;
            /// Quad-precision floating-point extension.
            const FLOAT128      = 1 << 16;
            /// Supervisor mode implemented.
            const SUPERVISOR    = 1 << 18;
            /// User mode implemented.
            const USER          = 1 << 20;
            /// Non-standard extensions present.
            const NON_STANDARD  = 1 << 23;
        }

    }

    /// Max instruction length for the ISA.
    #[derive(Debug, Clone, Copy)]
    #[repr(u8)]
    pub enum Mxlen {
        Len32 = 1,
        Len64 = 2,
        Len128 = 3,
    }

    #[derive(Debug)]
    pub struct Isa {
        pub mxlen: Mxlen,
        pub extensions: Extensions,
    }

    /// Get the information of the currently supported ISA.
    #[inline(always)]
    pub fn get() -> Option<Isa> {

        let raw: usize;
        unsafe { core::arch::asm!("csrr {0}, misa", out(reg) raw); }

        if raw == 0 {
            return None;
        }
        
        let raw_mxlen = raw >> (usize::BITS - 2);
        let raw_extensions = raw & ((1 << 26) - 1);
        
        Some(Isa {
            mxlen: match raw_mxlen {
                1 => Mxlen::Len32,
                2 => Mxlen::Len64,
                3 => Mxlen::Len128,
                _ => return None,
            },
            extensions: Extensions::from_bits_truncate(raw_extensions as u32)
        })

    }

}

pub mod mscratch {
    
    /// Set the `mscratch` register for the hart executing this function.
    #[inline(always)]
    pub fn set(value: usize) {
        unsafe { core::arch::asm!("csrw mscratch, {0}", in(reg) value); }
    }

}

pub mod mie {
    
    bitflags::bitflags! {

        pub struct MieFlags: u16 {
            /// Supervisor Software Interrupt Enable
            const SSIE = 1 << 1;
            /// Machine Software Interrupt Enable
            const MSIE = 1 << 3;
            /// Supervisor Timer Interrupt Enable
            const STIE = 1 << 5;
            /// Machine Timer Interrupt Enable
            const MTIE = 1 << 7;
            /// Supervisor External Interrupt Enable
            const SEIE = 1 << 9;
            /// Machine External Interrupt Enable
            const MEIE = 1 << 11;
        }

    }

    #[inline(always)]
    pub fn set(flags: MieFlags) {
        unsafe { core::arch::asm!("csrw mie, {0}", in(reg) flags.bits as u32); }
    }

}
