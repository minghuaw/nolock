use core::fmt;
use std::{intrinsics::transmute, mem, ptr::NonNull, usize};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Returns a bitmask containing the unused least significant bits of an aligned pointer to `T`.
#[inline]
fn low_bits<T>() -> usize {
    (1 << mem::align_of::<T>().trailing_zeros()) - 1
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
pub(crate) fn compose_tag<T>(data: usize, tag: usize) -> usize {
    let mask = low_bits::<T>();
    (data & !mask) | (tag & mask)
}

/// Decomposes a tagged pointer `data` into the pointer and the tag.
#[inline]
pub(crate) fn decompose_tag<T>(data: usize) -> (usize, usize) {
    let mask = low_bits::<T>();
    (data & !mask, data & mask)
}

/// Arc pointer that uses the lower unused bits for tagging
pub struct TaggedArc<T> {
    // data is a tagged pointer
    pub(crate) data: NonNull<T>,
}

unsafe impl<T: Sync + Send> Send for TaggedArc<T> {}
unsafe impl<T: Sync + Send> Sync for TaggedArc<T> {}

impl<T> TaggedArc<T> {
    pub fn new(val: impl Into<Arc<T>>) -> Self {
        let ptr = val.into();
        Self::from_arc(ptr)
    }

    pub fn compose(ptr: Arc<T>, tag: usize) -> Self {
        let ptr: Arc<T> = ptr.into();
        let raw = Arc::into_raw(ptr) as usize;
        let data = compose_tag::<T>(raw, tag);
        // SAFETY: data is composed from a valid pointer addr and tag
        let data = unsafe { NonNull::new_unchecked(data as *mut T) };
        Self {
            data,
        }
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let raw = Arc::into_raw(val) as *mut T;

        // SAFETY: pointer address obtained from a valid Arc pointer
        let data = unsafe { NonNull::new_unchecked(raw)};
        Self {
            data,
        }
    }

    pub fn into_arc(self) -> Arc<T> {
        // remove tag information
        let (data, _) = decompose_tag::<Arc<T>>(self.into_usize());
        unsafe { Arc::from_raw(data as *const T) }
    }

    pub fn decompose(ptr: TaggedArc<T>) -> (Arc<T>, usize) {
        let (data, tag) = decompose_tag::<Arc<T>>(
            // SAFETY: only valid pointers will be stored
            unsafe { transmute::<NonNull<T>, usize>(ptr.data) }    
        );
        let ptr = data as *const T;
        unsafe {
            (Arc::from_raw(ptr), tag)
        }
    }

    pub fn into_usize(self) -> usize {
        unsafe {
            transmute(self.data)
        }
    }

    /// # Safety
    /// 
    /// `usize` may not be a valid pointer address
    pub unsafe fn from_usize(data: usize) -> Option<Self> {
        let data = NonZeroUsize::new(data)?;
        let ret = Self {
            data: transmute(data)
        };
        Some(ret)
    }

    pub fn as_raw(&self) -> *const T {
        let (data, _) = decompose_tag::<Arc<T>>(
            unsafe { transmute::<NonNull<T>, usize>(self.data) }
        );
        data as *const T
    }

    pub unsafe fn from_raw(raw: *const T) -> Option<Self> {
        let data = raw as usize;
        Self::from_usize(data)
    }

    pub fn into_raw(ptr: TaggedArc<T>) -> *const T {
        ptr.as_raw()
    }

    pub fn tag(&self) -> usize {
        let (_, tag) = decompose_tag::<Arc<T>>(
            unsafe { transmute::<NonNull<T>, usize>(self.data) }
        );
        tag
    }

    pub fn with_tag(&self, tag: usize) -> Self {
        // `compose_tag` will take care of removing any old tag
        // that is already with the current self.data
        let data = compose_tag::<T>(
            unsafe { transmute(self.data) }, 
            tag
        );

        // SAFETY: `self.data` is already `NonZeroUsize`
        let data = unsafe { NonNull::new_unchecked(data as *mut T) };
        Self {
            data,
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
        let (data, tag) = decompose_tag::<Arc<T>>(
            unsafe { transmute::<NonNull<T>, usize>(self.data) }
        );
        let ptr = unsafe { Arc::from_raw(data as *const T) };
        let new = Arc::clone(&ptr);
        TaggedArc::new(new)
            .with_tag(tag)
    }
}

impl<T: fmt::Debug> fmt::Debug for TaggedArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (data, _) = decompose_tag::<Arc<T>>(
            unsafe { transmute::<NonNull<T>, usize>(self.data) }
        );       
        let ptr = unsafe { Arc::from_raw(data as *const T) };
        fmt::Debug::fmt(&ptr, f) 
    }
}

// impl<T> Drop for TaggedArc<T> {
//     fn drop(&mut self) {
//         let (data, _) = decompose_tag::<T>(self.data.get());
//         // ERROR: this will incur memory SIGSEV
//         let ptr = unsafe { Arc::from_raw(data as *const T) };        // manually call drop on Arc ptr
//         drop(ptr);
//     }
// }

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]
    use std::{mem::{size_of_val, transmute}, ptr::NonNull, sync::atomic::AtomicUsize};
    use std::sync::atomic::Ordering;

    use super::*;

    #[test]
    fn neighbor_ptr() {
        let p1 = Arc::new(1);
        let p2 = Arc::new(2);

        println!("{:p}", p1);
        println!("{:p}", p2);
    }

    #[test]
    fn test_raw_addr() {
        type Examining = Arc<i32>;
        let val = 23;

        let p1 = Arc::new(val);
        println!("Arc: {:p}", p1);
        let raw = Arc::into_raw(p1);
        println!("*const T: {:p}", raw);
        let data = raw as usize;
        println!("usize: 0x{:x}", data);
        let nzeros = data.trailing_zeros();
        println!("trailing zeros in bin: {:?}", nzeros);

        let align = std::mem::align_of::<Examining>();
        println!("align: {:?}", &align);
        let mask = low_bits::<Examining>();
        println!("low bits mask of Arc<&str>: {:?}", mask);
    }

    #[test]
    fn check_align() {
        let ptr = Arc::new("data");
        let align = std::mem::align_of_val(&ptr);
        println!("{:?}", align);
        let align = std::mem::align_of::<Arc<u8>>();
        println!("{:?}", align);
    }

    #[cfg(feature = "tag")]
    #[test]
    fn low_bits_of_arc() {
        let align = low_bits::<Arc<&str>>();
        println!("{:b}", &align);
        assert_eq!(align, (1 << 3) - 1 );
    }

    #[cfg(feature = "tag")]
    #[test]
    fn tag() {
        let ptr = Arc::new(1);
        let raw = Arc::into_raw(ptr) as usize;
        let tag = 0b01;
        let tagged = compose_tag::<Arc<&str>>(raw, tag);
        let (raw1, tag1) = decompose_tag::<Arc<&str>>(tagged);

        println!("raw: 0x{:x}", &raw);
        println!("tag: 0x{:x}", &tag);
        println!("tagged: 0x{:x}", &tagged);
        println!("raw1: 0x{:x}", &raw1);
        println!("tag1: 0x{:x}", &tag1);
        assert_eq!(raw, raw1);
        assert_eq!(tag, tag1);
    }

    #[cfg(feature = "tag")]
    #[test]
    fn compose_and_decompose() {
        let ptr = Arc::new(3);
        // let raw = Arc::into_raw(ptr);
        println!("{:p}", ptr);
        let tag = 0x01;
        let comp = TaggedArc::from_arc(ptr.clone()).with_tag(tag);
        let (out_ptr, out_tag) = TaggedArc::decompose(comp);
        // let (out_ptr, out_tag) = decompose_tag::<Box<i32>>(comp.data.get());
        println!("{:?}", out_ptr);
        // PANIC: This will panic because converting back from usize will make the pointer no longer valid 
        // for Arc
        // drop(comp);
        // let out_ptr = unsafe {
        //     Arc::from_raw(out_ptr as *const i32).clone()
        // };
        // let out_ptr = comp.into_arc();

        println!("{:p}", out_ptr);
        println!("{:?}", out_ptr);
        println!("{:?}", out_tag);
        assert_eq!(ptr, out_ptr);
        assert_eq!(tag, out_tag);
    }

    #[test]
    fn arc_into_and_from_raw() {
        let ptr = Arc::new(3);
        let raw = Arc::into_raw(ptr);
        // let raw = NonZeroUsize::new(raw as usize).unwrap();
        unsafe {
            // let prev = transmute::<&NonZeroUsize, &AtomicUsize>(&raw)
            //     .fetch_add(1, Ordering::Relaxed);
            // println!("0x{:x}", prev);

            // let prev = transmute::<&NonZeroUsize, &AtomicUsize>(&raw)
            //     .fetch_sub(1, Ordering::Relaxed);
            // println!("0x{:x}", prev);

            let mut a = 1;
            println!("{:p}", &a);
            a = a + 1;
            println!("{:p}", &a);            

            // let raw2 = transmute::<&AtomicUsize, &*const i32>(&data);
            // let raw2 = transmute::<usize, *const i32>(*data2);
            // let mask: usize = 0b111;
            let mask = low_bits::<Arc<i32>>();
            let tag = 0x01;
            let data = raw as usize;
            let data = data & !mask;
            let data = data | (tag & mask);
            // println!("{:?}", data);
            let data = data & !mask;
            // println!("{:?}", data);
            let raw = data as *const i32;

            // let ptr2 = Arc::from_raw(transmute::<NonZeroUsize, *mut i32>(raw));
            let ptr2 = Arc::from_raw(raw);
            println!("{:?}" , ptr2);
        }
    }

    #[test]
    fn test_compose_decompose_step_by_step() {
        let ptr = Arc::new(13);
        println!("[1] {:p}", ptr);
        // let raw = Arc::into_raw(ptr);
        // let data = raw as usize;
        let tag = 0x01;
        
        // compose tag
        // let mask = low_bits::<Arc<i32>>();
        // println!("{:?}", mask);
        // let comp = data & !mask;
        // println!("[2] 0x{:x}", &comp);
        // let comp = comp | (tag & mask);
        // let comp = compose_tag::<Arc<i32>>(data, tag);
        let comp = TaggedArc::compose(ptr.clone(), tag);
        println!("[3] 0x{:p}", &comp);

        // PROBLEM: two Arcs are constructed and one of them will
        // have problem dropping
        //
        // first try saving as an Arc
        // let _ptr = unsafe {
        //     Arc::from_raw(comp as *const i32)
        // };
        // println!("[4] {:p}", _ptr);
        // println!("[5] {:?} (Wrong value is expected)", _ptr);
        // drop(_ptr);

        // decompose
        // let data2 = comp & !mask;
        // let tag2 = comp & mask;
        // let (data2, tag2) = decompose_tag::<Arc<i32>>(comp);
        let (ptr2, tag2) = TaggedArc::decompose(comp);
        let data2 = Arc::into_raw(ptr2) as usize;
        println!("[6] 0x{:x}", data2);
        println!("[7] 0x{:x}", tag2);
        let raw2 = data2 as *const i32;

        let ptr2 = unsafe {
            // cast to Arc from decomposed pointers
            Arc::from_raw(raw2)
        };
        println!("[8] {:p}", ptr2);
        println!("[9] {:?}", ptr2);

        // error probably occured during dropping? No
        // drop(ptr);

        assert_eq!(ptr, ptr2);
        assert_eq!(tag, tag2);
    }

    #[test]
    fn test_size_of_ptrs() {
        let val = "12313231312321";
        let arc_ptr = Arc::new(val.clone());
        let box_ptr = Box::new(val.clone());
        
        println!("size(Arc) {:?}", size_of_val(&arc_ptr));
        println!("size(Box) {:?}", size_of_val(&box_ptr));
        
        let raw_arc = Arc::into_raw(arc_ptr);
        let raw_box = Box::into_raw(box_ptr);
        println!("size(raw Arc) {:?}", size_of_val(&raw_arc));
        println!("size(raw Box) {:?}", size_of_val(&raw_box));
    }
}