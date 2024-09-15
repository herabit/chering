use core::{
    fmt,
    hint::{assert_unchecked, unreachable_unchecked},
    marker::PhantomData,
    ptr::NonNull,
};

use sptr::{invalid_mut, Strict};

use crate::layout::HasLayout;

/// A tagged pointer that stores a tag in the alignment bits of a pointer.
///
/// This mainly exists as an internal type that seeks to, well, reduce
/// fuckups that can occur when doing pointer tagging manually.
#[repr(transparent)]
pub struct TagPtr<T> {
    raw: NonNull<T>,
}

impl<T> TagPtr<T> {
    /// Create a dangling tagged pointer to a `T`.
    #[inline]
    #[must_use]
    pub const fn dangling(tag: Tag<T>) -> TagPtr<T> {
        // SAFETY: Since `T::ALIGN` is never zero, `raw` is never null.
        let raw = invalid_mut(T::ALIGN.get() | tag.get());
        let raw = unsafe { NonNull::new_unchecked(raw) };

        TagPtr { raw }
    }

    /// Try to create a tagged pointer to a `T`.
    ///
    /// Returns `None` if `ptr` is not aligned for `T`.
    #[inline]
    #[must_use]
    pub fn try_new(ptr: NonNull<T>, tag: Tag<T>) -> Option<TagPtr<T>> {
        if ptr.is_aligned() {
            // SAFETY: Since `ptr` is aligned and not null, the tag will never conflict
            //         with the address, and additionally inserting the tag will never
            //         cause the address to be null.
            let raw = Strict::map_addr(ptr.as_ptr(), |addr| addr | tag.get());
            let raw = unsafe { NonNull::new_unchecked(raw) };

            Some(TagPtr { raw })
        } else {
            None
        }
    }

    /// Create a tagged pointer to a `T` without safety checks.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that `ptr` is properly aligned for `T`.
    #[inline]
    #[must_use]
    #[track_caller]
    pub unsafe fn new_unchecked(ptr: NonNull<T>, tag: Tag<T>) -> TagPtr<T> {
        match TagPtr::<T>::try_new(ptr, tag) {
            Some(ptr) => ptr,
            None if cfg!(debug_assertions) => panic!("`ptr` is not aligned"),
            // SAFETY: The caller ensures that `ptr` is aligned properly.
            None => unsafe { unreachable_unchecked() },
        }
    }

    /// Create a tagged pointer to a `T`.
    ///
    /// # Panics
    ///
    /// Panics if `ptr` is not properly aligned for `T`.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn new(ptr: NonNull<T>, tag: Tag<T>) -> TagPtr<T> {
        match TagPtr::<T>::try_new(ptr, tag) {
            Some(ptr) => ptr,
            None => panic!("`ptr` is not aligned"),
        }
    }

    /// Get the tag.
    #[inline]
    #[must_use]
    pub fn tag(self) -> Tag<T> {
        let tag = Strict::addr(self.raw.as_ptr()) & T::TAG_MASK;

        Tag::new(tag).unwrap()
    }

    /// Get the actual pointer.
    ///
    /// # Safety
    ///
    /// - The pointer is guaranteed to be properly aligned for `T`.
    #[inline]
    #[must_use]
    pub fn ptr(self) -> NonNull<T> {
        let ptr = Strict::map_addr(self.raw.as_ptr(), |addr| addr & T::PTR_MASK);

        unsafe {
            // SAFETY: Creating a tagged pointer requires that the pointer without the tag
            //         is properly aligned for `T` and non-null.
            assert_unchecked(ptr.is_aligned());

            NonNull::new_unchecked(ptr)
        }
    }

    /// Create a new tagged pointer with the same address as `self`.
    ///
    /// # Safety
    ///
    /// - The resulting pointer has the same provenance as `self`.
    #[inline]
    #[must_use]
    pub fn with_tag(self, tag: Tag<T>) -> TagPtr<T> {
        // SAFETY: We already know that `ptr` is valid, as it must be in order to construct
        //         a tagged pointer.
        unsafe { TagPtr::new_unchecked(self.ptr(), tag) }
    }

    /// Create a new tagged pointer with the same tag as `self`.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that `ptr` is properly aligned for `T`.
    /// - The resulting pointer has the same provenance as `ptr`, not `self`.
    #[inline]
    #[must_use]
    pub unsafe fn with_ptr(self, ptr: NonNull<T>) -> TagPtr<T> {
        debug_assert!(ptr.is_aligned(), "`ptr` is not aligned");

        // SAFETY: The caller ensures that `ptr` is valid.
        unsafe { TagPtr::new_unchecked(ptr, self.tag()) }
    }

    /// Update the tag.
    #[inline]
    #[must_use]
    pub fn set_tag(&mut self, tag: Tag<T>) {
        *self = self.with_tag(tag);
    }

    /// Update the pointer.
    ///
    /// # Safety.
    ///
    /// - The caller must ensure that `ptr` is properly aligned for `T`.
    /// - `self` will have the same provenance as `ptr` after this call.
    #[inline]
    #[must_use]
    pub unsafe fn set_ptr(&mut self, ptr: NonNull<T>) {
        *self = self.with_ptr(ptr);
    }
}

impl<T> fmt::Debug for TagPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TagPtr")
            .field("ptr", &self.ptr())
            .field("tag", &self.tag())
            .finish()
    }
}

impl<T> fmt::Pointer for TagPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ptr().fmt(f)
    }
}

impl<T> Clone for TagPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TagPtr<T> {}

impl<T> PartialEq for TagPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }

    #[inline]
    fn ne(&self, other: &Self) -> bool {
        self.raw != other.raw
    }
}

impl<T> Eq for TagPtr<T> {}

impl<T> PartialOrd for TagPtr<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for TagPtr<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T> core::hash::Hash for TagPtr<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state)
    }
}

/// A type that will always fit in the alignment bits of a pointer to `T`.
#[repr(transparent)]
pub struct Tag<T> {
    tag: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Tag<T> {
    pub const MIN: Tag<T> = Tag {
        tag: 0,
        _marker: PhantomData,
    };

    pub const MAX: Tag<T> = Tag {
        tag: <T as HasLayout>::TAG_MASK,
        _marker: PhantomData,
    };

    /// Returns whether a provided tag is valid.
    #[inline]
    #[must_use]
    pub const fn is_valid(tag: usize) -> bool {
        (tag & T::TAG_MASK) == tag
    }

    /// Try to create a tag, returning `None` if the tag
    /// does not fit within the alignment bits.
    #[inline]
    #[must_use]
    pub const fn new(tag: usize) -> Option<Tag<T>> {
        if Tag::<T>::is_valid(tag) {
            Some(Tag {
                tag,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Create a tag without checking that it fits in the alignment bits.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const unsafe fn new_unchecked(tag: usize) -> Tag<T> {
        match Tag::<T>::new(tag) {
            Some(tag) => tag,
            None if cfg!(debug_assertions) => panic!("tag does not fit within alignment bits"),
            None => unsafe { unreachable_unchecked() },
        }
    }

    /// Get the integer representation of this tag.
    #[inline]
    #[must_use]
    pub const fn get(self) -> usize {
        // SAFETY: Creating a tag requires that it always meets `Self::is_valid`.
        unsafe { assert_unchecked(Tag::<T>::is_valid(self.tag)) };

        self.tag
    }
}

impl<T> Default for Tag<T> {
    #[inline]
    fn default() -> Self {
        Self::MIN
    }
}

impl<T> Clone for Tag<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Tag<T> {}

impl<T1, T2> PartialEq<Tag<T2>> for Tag<T1> {
    #[inline]
    fn eq(&self, other: &Tag<T2>) -> bool {
        self.tag == other.tag
    }

    #[inline]
    fn ne(&self, other: &Tag<T2>) -> bool {
        self.tag != other.tag
    }
}

impl<T> PartialEq<usize> for Tag<T> {
    #[inline]
    fn eq(&self, other: &usize) -> bool {
        self.tag == *other
    }

    #[inline]
    fn ne(&self, other: &usize) -> bool {
        self.tag != *other
    }
}

impl<T> Eq for Tag<T> {}

impl<T1, T2> PartialOrd<Tag<T2>> for Tag<T1> {
    #[inline]
    fn partial_cmp(&self, other: &Tag<T2>) -> Option<core::cmp::Ordering> {
        Some(self.tag.cmp(&other.tag))
    }
}

impl<T> PartialOrd<usize> for Tag<T> {
    #[inline]
    fn partial_cmp(&self, other: &usize) -> Option<core::cmp::Ordering> {
        Some(self.tag.cmp(other))
    }
}

impl<T> Ord for Tag<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.tag.cmp(&other.tag)
    }
}

impl<T> core::hash::Hash for Tag<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.tag.hash(state)
    }
}

impl<T> fmt::Debug for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tag").field(&self.tag).finish()
    }
}

impl<T> fmt::Display for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl<T> fmt::Binary for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl<T> fmt::UpperHex for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl<T> fmt::LowerHex for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl<T> fmt::UpperExp for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl<T> fmt::LowerExp for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}
