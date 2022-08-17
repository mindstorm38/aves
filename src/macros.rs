//! Common macros definitions.

#[macro_export]
macro_rules! print {
    ($($arg:tt)+) => {
        #[allow(unused_unsafe)]
        {
            use core::fmt::Write;
            let _ = write!(unsafe { &mut crate::uart::DEFAULT }, $($arg)+);
        }
    };
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::print!("\n");
    }};
    ($fmt:expr) => ({
		$crate::print!(concat!($fmt, "\n"));
	});
	($fmt:expr, $($arg:tt)+) => ({
		$crate::print!(concat!($fmt, "\n"), $($arg)+);
	});
}

/// Write a formatted string to a slice.
#[macro_export]
macro_rules! write_slice {
    ($slice:expr, $($arg:tt)+) => {{
        use core::fmt::Write;
        struct WriteSliceCursor<'a>(&'a mut [u8]);
        impl Write for WriteSliceCursor<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                if s.len() > self.0.len() {
                    Err(core::fmt::Error)
                } else {
                    self.0[..s.len()].copy_from_slice(s.as_bytes());
                    Ok(())
                }
            }
        }
        write!(WriteSliceCursor($slice), $($arg)+)
    }};
}

#[macro_export]
macro_rules! mmio_struct {
    (
        $(
            $vis:vis struct $name:ident {
                $([$field_index:literal] $field_mod:ident $field_name:ident : $field_type:ty),*
                $(,)?
            }
        )*
    ) => {
        $(
            #[derive(Debug, Clone, Copy)]
            $vis struct $name(pub *mut u8);
            impl $name {

                #[inline]
                pub const fn new(ptr: *mut u8) -> Self {
                    Self(ptr)
                }

                $($crate::__mmio_struct_field!($field_mod, $field_name, $field_type, $field_index);)*

            }
        )*
    };
}

#[macro_export]
macro_rules! __mmio_struct_field {
    (r, $field_name:ident, $field_type:ty, $field_index:literal) => {
        #[inline]
        pub fn $field_name(&self) -> $field_type {
            unsafe { (self.0.add($field_index) as *mut $field_type).read_volatile() }
        }
    };
    (w, $field_name:ident, $field_type:ty, $field_index:literal) => {
        #[inline]
        pub fn $field_name(&self, value: $field_type) {
            unsafe { (self.0.add($field_index) as *mut $field_type).write_volatile(value) }
        }
    };
    (sub, $field_name:ident, $field_type:ty, $field_index:literal) => {
        #[inline]
        pub fn $field_name(&self) -> $field_type {
            <$field_type>::new(unsafe { self.0.add($field_index) })
        }
    };
}
