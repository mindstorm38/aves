//! Various utilities used accross the kernel.

use core::mem::{size_of, align_of, forget, needs_drop, ManuallyDrop};
use core::ptr::{copy_nonoverlapping, drop_in_place};


/// A fixed-size box for storing opaque type.
/// 
/// This type has a C-layout with the alignment of the pointer size.
/// The data put inside can't not have greater alignment.
/// 
/// *This structure is made for optimized low-level storages where
/// the metadata about the inner type is know at compile time,
/// because it's eraised when put in this structure.*
/// 
/// **This structure will drop the given value if needed by its type.**.
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))] // FIXME: These attrs might be useless, because of the function pointer internally constraining the alignment.
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
pub struct OpaqueBox<const SIZE: usize> {
    data: ManuallyDrop<[u8; SIZE]>,
    data_drop: Option<fn(*mut u8)>,
}

impl<const SIZE: usize> OpaqueBox<SIZE> {

    pub fn new<D>(data: D) -> Self {

        assert!(size_of::<D>() <= SIZE, "given data is too big to fit in {SIZE} bytes of this opaque box");
        assert!(align_of::<D>() <= size_of::<usize>(), "given data has an aligment greater than target pointer size {}", size_of::<usize>());

        let mut final_data = [0; SIZE];

        unsafe {

            // Here we get a pointer to the data given in parameter.
            // Then we copy the given data into the internal byte array.
            let raw_data = &data as *const D as *const u8;
            copy_nonoverlapping(raw_data, final_data.as_mut_ptr(), size_of::<D>());

            // We just moved the data into the internal allocation, 
            // so we forget the given data.
            forget(data);

        }

        Self {
            data: ManuallyDrop::new(final_data),
            data_drop: needs_drop::<D>().then_some(<D as DataDrop>::drop)
        }

    }

    /// Get a pointer to the internal data, assuming the given generic type.
    #[inline]
    pub fn as_ptr<D>(&self) -> *const D {
        self.data.as_ptr() as *const _
    }

    /// Get a mutable pointer to the internal data, assuming the given generic type.
    #[inline]
    pub fn as_mut_ptr<D>(&mut self) -> *mut D {
        self.data.as_mut_ptr() as *mut _
    }

}

impl<const SIZE: usize> Drop for OpaqueBox<SIZE> {
    fn drop(&mut self) {
        if let Some(data_drop) = self.data_drop {
            data_drop(self.data.as_mut_ptr());
        }
    }
}


/// Internal trait used to implement a single generic
/// drop implementation for each existing sized type.
trait DataDrop {

    /// Drop the type implementing this trait.
    /// **The given pointer should point to a properly aligned, sized
    /// and not yet dropped instance of the implementing type.**
    fn drop(ptr: *mut u8);

}

impl<T> DataDrop for T {
    fn drop(ptr: *mut u8) {
        unsafe { drop_in_place(ptr as *mut T); }
    }
}
