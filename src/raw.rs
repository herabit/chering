use core::sync::atomic::AtomicUsize;

use crate::tag_ptr::TagPtr;

/// Header for a shared vec.
#[repr(C)]
pub struct Header<T> {
    /// Capacity of the vector.
    pub cap: usize,
    /// Length of the vector.
    pub len: usize,
    /// Reference count.
    pub ref_count: AtomicUsize,
    /// Start of the data in the vector.
    pub data: [T; 0],
}

/// Pointer to the start of the data in a shared vec.
#[repr(transparent)]
pub struct Data<T> {
    /// Despite this saying a tagged pointer to `Header<T>`, it is actually a pointer to
    /// the data field in the header.
    ptr: TagPtr<Header<T>>,
}
