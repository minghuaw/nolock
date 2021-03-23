use std::{marker::PhantomPinned, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use std::marker::PhantomData;
use std::mem;

/// Returns a bitmask containing the unused least significant bits of an aligned pointer to `T`.
#[inline]
fn low_bits<T>() -> usize {
    (1 << mem::align_of::<T>()) - 1
}

/// Panics if the pointer is not properly unaligned.
#[inline]
fn ensure_aligned<T>(raw: usize) {
    assert_eq!(raw & low_bits::<T>(), 0, "unaligned pointer");
}

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

// pub struct Tag<P>(usize, PhantomData<P>);
// impl<P> Tag<P> {
//     /// Creates tag by applying a bit mask that only keeps the the low bits 
//     /// `usize` type.
//     pub fn new(tag: usize) -> Self {
//         let tag = tag & low_bits::<P>();
//         Self(tag, PhantomData)
//     }
// }

// impl<P> From<usize> for Tag<P> {
//     fn from(val: usize) -> Self {
//         Self::new(val)
//     }
// }

// impl<P> From<Tag<P>> for usize {
//     fn from(tag: Tag<P>) -> Self {
//         tag.0
//     }
// }

pub struct TaggedArc<T> {
    data: usize,
    _marker: PhantomData<T>
}

impl<T> TaggedArc<T> {
    pub fn new(val: impl Into<Arc<T>>) -> Self {
        let ptr = val.into();
        Self::from_arc(ptr)
    }

    pub fn new_with_tag(val: impl Into<Arc<T>>, tag: usize) -> Self {
        let ptr: Arc<T> = val.into();
        let raw = Arc::into_raw(ptr) as usize;
        let data = compose_tag::<T>(raw, tag);
        unsafe {
            Self::from_usize(data)
        }
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let data = Arc::into_raw(val) as usize;
        Self {
            data,
            _marker: PhantomData
        }
    }

    pub fn into_arc(self) -> Arc<T> {
        // remove tag information
        let(data, _) = decompose_tag::<T>(self.data);
        let ptr = data as *const T;
        unsafe {
            Arc::from_raw(ptr)
        }
    }

    pub fn into_usize(self) -> usize {
        self.data
    }

    /// # Safety
    /// 
    /// `data` may not be a valid pointer
    pub unsafe fn from_usize(data: usize) -> Self {
        Self {
            data,
            _marker: PhantomData
        }
    }

    pub fn as_raw(&self) -> *const T {
        let (data, _) = decompose_tag::<T>(self.data);
        data as *const T
    }

    pub unsafe fn from_raw(raw: *const T) -> Self {
        let data = raw as usize;
        Self::from_usize(data)
    }

    pub fn into_raw(self) -> *const T {
        self.as_raw()
    }

    pub fn tag(&self) -> usize {
        let (_, tag) = decompose_tag::<T>(self.data);
        tag
    }

    pub fn with_tag(&self, tag: usize) -> Self {
        let data = compose_tag::<T>(self.data, tag);
        Self {
            data,
            _marker: PhantomData
        }
    }
}

// impl<T> From<Arc<T>> for TaggedArc<T> {
//     fn from(val: Arc<T>) -> Self {
//         let ptr: Arc<T> = val.into();
//         Self::from_arc(ptr)
//     }
// }

impl<T, P: Into<Arc<T>>> From<P> for TaggedArc<T> {
    fn from(val: P) -> Self {
        let ptr: Arc<T> = val.into();
        Self::from_arc(ptr)
    }
}

// impl<T> From<TaggedArc<T>> for Arc<T> {
//     fn from(tagged: TaggedArc<T>) -> Self {
//         tagged.into_arc()
//     }
// }

impl<T> Clone for TaggedArc<T> {
    fn clone(&self) -> Self {
        let (raw, tag) = decompose_tag::<T>(self.data);
        let ptr: Arc<T> = unsafe {
            Arc::from_raw(raw as *const T)
        };
        let new = Arc::clone(&ptr);
        let new_data = Arc::into_raw(new) as usize;
        let tagged_data = compose_tag::<T>(new_data, tag);
        unsafe {
            Self::from_usize(tagged_data)
        }
    }
}

/// A wrapper that change all API to only accept and return `Arc` and allows tagging
pub struct AtomicArc<T> {
    data: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T> AtomicArc<T> {
    pub fn new<P: Into<Arc<T>>>(val: P) -> Self {
        let ptr: Arc<T> = val.into();
        Self::from_arc(ptr)
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let data = Arc::into_raw(val) as usize;
        unsafe {
            Self::from_usize(data)
        }
    }

    pub fn from_tagged(tagged: TaggedArc<T>) -> Self {
        let data = tagged.into_usize();
        unsafe {
            Self::from_usize(data)
        }
    }

    // Only API that expose Arc should be public
    unsafe fn from_usize(val: usize) -> Self {
        let data = AtomicUsize::new(val);
        Self {
            data,
            _marker: PhantomData
        }
    }

    pub fn get_mut() {
        unimplemented!()
    }

    /// Loads a value from the atomic pointer.
    ///
    /// `load` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Acquire` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Release` or `AcqRel`.
    pub fn load(&self, order: Ordering) -> TaggedArc<T> {
        let data = self.data.load(order);
        unsafe {
            TaggedArc::from_usize(data)
        }
    }

    /// Stores a value into the pointer
    ///
    /// `store` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Release` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Acquire` or `AcqRel`.
    pub fn store<P: Into<TaggedArc<T>>>(&self, val: P, order: Ordering) {
        let ptr: TaggedArc<T> = val.into();
        let new_data = ptr.into_usize();
        self.data.store(new_data, order)
    }

    /// 
    pub fn swap<P: Into<TaggedArc<T>>>(&self, val: P, order: Ordering) -> TaggedArc<T> {
        let ptr: TaggedArc<T> = val.into();
        let new_data = ptr.into_usize();
        let old_data = self.data.swap(new_data, order);
        
        // SAFETY: only raw Arc pointers will be stored in the pointer
        unsafe {
            TaggedArc::from_usize(old_data)
        }
    }

    pub fn compare_exchange(
        &self,
        current: impl Into<TaggedArc<T>>,
        new: impl Into<TaggedArc<T>>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<TaggedArc<T>, TaggedArc<T>> {
        let current: TaggedArc<T> = current.into();
        let current = current.into_usize();
        let new: TaggedArc<T> = new.into();
        let new = new.into_usize();
        self.data.compare_exchange(current, new, success, failure)
            .map(|success| {
                unsafe {TaggedArc::from_usize(success)}
            })
            .map_err(|failure| {
                unsafe {TaggedArc::from_usize(failure)}
            })
    }

    pub fn compare_exchange_weak(
        &self,
        current: impl Into<TaggedArc<T>>,
        new: impl Into<TaggedArc<T>>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<TaggedArc<T>, TaggedArc<T>> {
        let current: TaggedArc<T> = current.into();
        let current = current.into_usize();
        let new: TaggedArc<T> = new.into();
        let new = new.into_usize();
        self.data.compare_exchange_weak(current, new, success, failure)
            .map(|success| {
                unsafe{ TaggedArc::from_usize(success) }
            })
            .map_err(|failure| {
                unsafe{ TaggedArc::from_usize(failure) }
            })
    }

    /// Similar to the same function in `crossbeam_epoch::Atomic`
    pub fn fetch_and() {
        unimplemented!()
    }

    /// Similar to the same function in `crossbeam_epoch::Atomic`
    pub fn fetch_or() {
        unimplemented!()
    }

    /// Similar to the same function in `crossbeam_epoch::Atomic`
    pub fn fetch_xor() {
        unimplemented!()
    }
}

impl<T> From<Arc<T>> for AtomicArc<T> {
    fn from(val: Arc<T>) -> Self {
        Self::from_arc(val)
    }
}

impl<T> From<T> for AtomicArc<T> {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T> Clone for AtomicArc<T> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_bits_of_arc() {
        let align = low_bits::<Arc<i8>>();
        println!("{:b}", &align);
        assert_eq!(align, (1 << 8) - 1 );
    }

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