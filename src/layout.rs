use core::{alloc::Layout, mem, num::NonZeroUsize};

/// Helper trait for quickly obtaining the memory layout information of a type.
///
/// This exists mainly for assisting pointer tagging, but is just generally useful.
pub trait HasLayout: Sized {
    /// The size of the type.
    const SIZE: usize = mem::size_of::<Self>();

    /// The alignment of the type.
    const ALIGN: NonZeroUsize = match NonZeroUsize::new(mem::align_of::<Self>()) {
        Some(align) => align,
        None => panic!("the alignment is somehow zero"),
    };

    /// The memory layout of the type.
    const LAYOUT: Layout = Layout::new::<Self>();

    /// The amount of bits alignment bits for a given pointer to [`Self`].
    const ALIGN_BITS: u32 = mem::align_of::<Self>().trailing_zeros();

    /// Whether or not any properly aligned pointer to [`Self`] can
    /// be used for pointer tagging.
    const TAGGING_ALLOWED: bool = <Self as HasLayout>::ALIGN_BITS != 0;

    /// A bitmask of the alignment bits for a pointer of [`Self`].
    const ALIGN_MASK: usize = {
        let shift = usize::BITS - <Self as HasLayout>::ALIGN_BITS;

        // It is lowkey stupid that you cannot shift more than `BITS - 1` bits.
        if shift < usize::BITS {
            usize::MAX >> shift
        } else {
            0
        }
    };

    /// An alias for [`HasLayout::ALIGN_MASK`] that makes it clearer that
    /// the mask is used for tagging.
    const TAG_MASK: usize = <Self as HasLayout>::ALIGN_MASK;

    /// A bitmask of the bits actually used to address a pointer of [`Self`].
    const PTR_MASK: usize = !<Self as HasLayout>::TAG_MASK;
}

impl<T> HasLayout for T {}
