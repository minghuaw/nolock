use std::sync::{Arc, atomic::AtomicUsize};
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

pub struct AtomicArc<T> {
    data: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T> AtomicArc<T> {
    pub fn new(val: T) -> Self {
        let val = Arc::new(val);
        Self::from_arc(val)
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let data = Arc::into_raw(val) as usize;
        Self::from_usize(data)
    }

    // Only API that expose Arc should be public
    fn from_usize(val: usize) -> Self {
        let data = AtomicUsize::new(val);
        Self {
            data,
            _marker: PhantomData
        }
    }

    pub fn get_mut() {
        unimplemented!()
    }

    pub fn into_inner(self) -> *mut T {
        let data = self.data.into_inner();
        data as *mut T
    }

    pub fn load() {
        unimplemented!()
    }

    pub fn store() {
        unimplemented!()
    }

    pub fn swap() {
        unimplemented!()
    }

    pub fn compare_exchange() {
        unimplemented!()
    }

    pub fn compare_exchange_weak() {
        unimplemented!()
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