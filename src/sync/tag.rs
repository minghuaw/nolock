use std::mem;
use std::borrow::Borrow;
use std::num::NonZeroUsize;
use std::marker::PhantomData;
use std::sync::Arc;

/// Returns a bitmask containing the unused least significant bits of an aligned pointer to `T`.
#[inline]
fn low_bits<T>() -> usize {
    (1 << mem::align_of::<T>()) - 1
}

// /// Panics if the pointer is not properly unaligned.
// #[allow(dead_code)]
// #[inline]
// fn ensure_aligned<T>(raw: usize) {
//     assert_eq!(raw & low_bits::<T>(), 0, "unaligned pointer");
// }

/// Given a tagged pointer `data`, returns the same pointer, but tagged with `tag`.
///
/// `tag` is truncated to fit into the unused bits of the pointer to `T`.
#[inline]
fn compose_tag<T>(data: usize, tag: usize) -> usize {
    (data & !low_bits::<T>()) | (tag & low_bits::<T>())
}

/// Decomposes a tagged pointer `data` into the pointer and the tag.
#[inline]
fn decompose_tag<T>(data: usize) -> (usize, usize) {
    (data & !low_bits::<T>(), data & low_bits::<T>())
}

/// Arc pointer that uses the lower unused bits for tagging
pub struct TaggedArc<T> {
    pub(crate) data: NonZeroUsize,
    _marker: PhantomData<T>
}

unsafe impl<T: Sync + Send> Send for TaggedArc<T> {}
unsafe impl<T: Sync + Send> Sync for TaggedArc<T> {}

impl<T> TaggedArc<T> {
    pub fn new(val: impl Into<Arc<T>>) -> Self {
        let ptr = val.into();
        Self::from_arc(ptr)
    }

    pub fn compose(val: impl Into<Arc<T>>, tag: usize) -> Self {
        let ptr: Arc<T> = val.into();
        let raw = Arc::into_raw(ptr) as usize;
        let data = compose_tag::<T>(raw, tag);
        // SAFETY: data is composed from a valid pointer addr and tag
        let data = unsafe { NonZeroUsize::new_unchecked(data) };
        Self {
            data,
            _marker: PhantomData,
        }
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let data = Arc::into_raw(val) as usize;

        // SAFETY: pointer address obtained from a valid Arc pointer
        let data = unsafe { NonZeroUsize::new_unchecked(data) };
        
        Self {
            data,
            _marker: PhantomData
        }
    }

    pub fn into_arc(self) -> Arc<T> {
        // remove tag information
        let (ptr, _) = Self::decompose(self);
        ptr
    }

    pub fn decompose(ptr: impl Borrow<TaggedArc<T>>) -> (Arc<T>, usize) {
        let (data, tag) = decompose_tag::<T>(ptr.borrow().data.get());
        let ptr = data as *const T;
        unsafe {
            (Arc::from_raw(ptr), tag)
        }
    }

    pub fn into_usize(self) -> usize {
        self.data.get()
    }

    /// # Safety
    /// 
    /// `usize` may not be a valid pointer address
    pub unsafe fn from_usize(data: usize) -> Option<Self> {
        let data = NonZeroUsize::new(data)?;
        let ret = Self {
            data,
            _marker: PhantomData
        };
        Some(ret)
    }

    pub fn as_raw(&self) -> *const T {
        let (data, _) = decompose_tag::<T>(self.data.get());
        data as *const T
    }

    pub unsafe fn from_raw(raw: *const T) -> Option<Self> {
        let data = raw as usize;
        Self::from_usize(data)
    }

    pub fn into_raw(self) -> *const T {
        self.as_raw()
    }

    pub fn tag(&self) -> usize {
        let (_, tag) = decompose_tag::<T>(self.data.get());
        tag
    }

    pub fn with_tag(&self, tag: usize) -> Self {
        // `compose_tag` will take care of removing any old tag
        // that is already with the current self.data
        let data = compose_tag::<T>(self.data.get(), tag);

        // SAFETY: `self.data` is already `NonZeroUsize`
        let data = unsafe { NonZeroUsize::new_unchecked(data) };
        Self {
            data,
            _marker: PhantomData
        }
    }
}

impl<T> From<Arc<T>> for TaggedArc<T> {
    fn from(ptr: Arc<T>) -> Self {
        Self::from_arc(ptr)
    }
}

impl<T> From<TaggedArc<T>> for Arc<T> {
    fn from(ptr: TaggedArc<T>) -> Self {
        ptr.into_arc()
    }
}

impl<T> Clone for TaggedArc<T> {
    fn clone(&self) -> Self {
        // let (raw, tag) = decompose_tag::<T>(self.data);
        let (ptr, tag) = Self::decompose(self);
        let new = Arc::clone(&ptr);
        TaggedArc::compose(new, tag)
    }
}

impl<T> Drop for TaggedArc<T> {
    fn drop(&mut self) {
        let (ptr, _) = Self::decompose(self);
        // manually call drop on Arc ptr
        drop(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "tag")]
    #[test]
    fn low_bits_of_arc() {
        let align = low_bits::<Arc<i8>>();
        println!("{:b}", &align);
        assert_eq!(align, (1 << 8) - 1 );
    }

    #[cfg(feature = "tag")]
    #[test]
    fn tag() {
        let ptr = Arc::new(1i32);
        let raw = Arc::into_raw(ptr) as usize;
        let tag = 0b01;
        let tagged = compose_tag::<i32>(raw, tag);
        let (raw1, tag1) = decompose_tag::<i32>(tagged);

        println!("raw: 0x{:x}", &raw);
        println!("tag: 0x{:x}", &tag);
        println!("tagged: 0x{:x}", &tagged);
        println!("raw1: 0x{:x}", &raw1);
        println!("tag1: 0x{:x}", &tag1);
        assert_eq!(raw, raw1);
        assert_eq!(tag, tag1);
    }
}