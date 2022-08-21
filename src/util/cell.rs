use core::mem::{size_of, align_of, needs_drop};
use core::ptr::drop_in_place;


/// A fixed-size box for storing opaque type.
/// 
/// This type has a C-layout with the alignment of the pointer size.
/// The data put inside can't have greater alignment.
/// 
/// *This structure is made for optimized low-level storages where
/// the metadata about the inner type is know at compile time,
/// because it's erased when put in this structure.*
/// 
/// **This structure will drop the given value if needed by its type.**.
#[repr(C)]
pub struct OpaqueCell<const SIZE: usize> {
    data: [u8; SIZE],
    /// This optional function pointer has the size of a function pointer, 
    /// which will force alignment of the structure to the pointer size.
    /// 
    /// Note that, from the [`unsafe code guidelines`], this has the same
    /// layout as the function pointer itself.
    /// 
    /// [`unsafe code guidelines`]: https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html
    data_drop: Option<fn(*mut u8)>,
}

impl<const SIZE: usize> OpaqueCell<SIZE> {

    pub fn new<D>(data: D) -> Self {

        assert!(size_of::<D>() <= SIZE, "given data is too big to fit in {SIZE} bytes of this opaque box");
        assert!(align_of::<D>() <= size_of::<usize>(), "given data has an aligment greater than target pointer size {}", size_of::<usize>());

        let mut final_data = [0; SIZE];

        unsafe {
            // Move data into the byte array.
            core::ptr::write(final_data.as_mut_ptr() as *mut D, data);
        }

        fn data_drop<T>(ptr: *mut u8) {
            unsafe { drop_in_place(ptr as *mut T); }
        }

        Self {
            data: final_data,
            data_drop: needs_drop::<D>().then_some(data_drop::<D>)
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

impl<const SIZE: usize> Drop for OpaqueCell<SIZE> {
    fn drop(&mut self) {
        if let Some(data_drop) = self.data_drop {
            data_drop(self.data.as_mut_ptr());
        }
    }
}
