//! A special module used to configure the kernel at compile-
//! time. Specificaly for kernel drivers to use and their 
//! configuration.


/// A macro to easily register drivers.
macro_rules! drivers {
    ($($name:ident: $typ:ty = $constructor:expr;)*) => {

        use crate::driver::*;

        $(static $name: $typ = $constructor;)*

        pub static DRIVERS: [&'static dyn Driver; $crate::count!($($name)*)] = [
            $(&$name),*
        ];

    };
}


drivers! {
    BLOCK: BlockDriver = BlockDriver::new();
    VIRTIO: VirtioDriver<0x1000_1000, 0x1000, 8> = VirtioDriver::new().with_block(&BLOCK);
}
