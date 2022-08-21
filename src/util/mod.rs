//! Various utilities used accross the kernel.

mod cell;
pub use cell::OpaqueCell;




// THE FOLLOWING IS A PROOF OF CONCPET FAT DYNAMIC REFERENCE.
// This avoid one level of indirection, for example, a standard
// dyn pointer is two word wide and performs the following
// indirections: caller -> vtable -> function.
// Here we avoid the vtable step, because our pointer is actually
// 3 word wide, but the vtable indirection is skept because we
// directly store the functions pointers in the fat pointer.

// pub trait DynTest {
//     fn a(&self);
//     fn b(&self) -> u32;
//     fn c(&mut self);
// }

// pub struct FatDyn {
//     ptr: *const (),
//     a: fn(*const ()),
//     b: fn(*const ()) -> u32,
// }

// impl FatDyn {

//     pub const fn new<T: DynTest>(ptr: &'static T) -> Self {
//         Self {
//             ptr: ptr as *const T as *const _,
//             a: unsafe { transmute::<fn(&'static T), _>(T::a) },
//             b: unsafe { transmute::<fn(&'static T) -> u32, _>(T::b) },
//         }
//     }

//     #[inline]
//     pub fn a(&self) {
//         (self.a)(self.ptr);
//     }

//     #[inline]
//     pub fn b(&self) -> u32 {
//         (self.b)(self.ptr)
//     }

// }
