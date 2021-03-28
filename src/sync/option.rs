use std::{intrinsics::transmute, mem::transmute_copy, sync::{Arc, atomic::AtomicUsize}};
use std::sync::atomic::Ordering;

use super::Atomic;

#[cfg(feature = "tag")]
use super::TaggedArc;

#[cfg(feature = "tag")]
impl<T> Atomic for Option<TaggedArc<T>> {
    type Elem = TaggedArc<T>;

    fn load(&self, order: Ordering) -> Self::Elem {
        let ptr = unsafe {
            let addr = transmute_copy::<Self, AtomicUsize>(self)
                .load(order);
            TaggedArc::from_usize(addr)
                .expect("TaggedArc cannot be Null")
        };
        // clone because `load` does not give away ownership
        TaggedArc::clone(&ptr)
    }

    fn store(&self, val: impl Into<Self::Elem>, order: Ordering) {
        let ptr: Self::Elem = val.into();
        let new_data = ptr.into_usize();

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .store(new_data, order)
        }
    }

    fn swap(&self, val: impl Into<Self::Elem>, order: Ordering) -> Self::Elem {
        let ptr: Self::Elem = val.into();
        let new_data = ptr.into_usize();

        unsafe {
            let old_data = transmute::<&Self, &AtomicUsize>(self)
                .swap(new_data, order);
            TaggedArc::from_usize(old_data)
                .expect("TaggedArc cannot be Null")
        }
    }

    fn compare_exchange(&self, current: impl Into<Self::Elem>, new: impl Into<Self::Elem>, success: Ordering, failure: Ordering) -> Result<Self::Elem, Self::Elem> {
        let current: Self::Elem = current.into();
        let current = current.into_usize();
        let new: Self::Elem = new.into();
        let new = new.into_usize();

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    TaggedArc::from_usize(ok)
                        .expect("TaggedArc cannot be Null")
                })
                .map_err(|err| {
                    TaggedArc::from_usize(err)
                        .expect("TaggedArc cannot be Null")
                })
        }
    }

    fn compare_exchange_weak(&self, current: impl Into<Self::Elem>, new: impl Into<Self::Elem>, success: Ordering, failure: Ordering) -> Result<Self::Elem, Self::Elem> {
        let current: Self::Elem = current.into();
        let current = current.into_usize();
        let new: Self::Elem = new.into();
        let new = new.into_usize();

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    TaggedArc::from_usize(ok)
                        .expect("TaggedArc cannot be Null")
                })
                .map_err(|err| {
                    TaggedArc::from_usize(err)
                        .expect("TaggedArc cannot be Null")
                })
        }
    }
}

impl<T> Atomic for Option<Arc<T>> {
    type Elem = Arc<T>;

    fn load(&self, order: Ordering) -> Self::Elem {
        let ptr = unsafe {
            let addr = transmute_copy::<Self, AtomicUsize>(self)
                .load(order);
            Arc::from_raw(addr as *const T) 
        };
        // clone because `load` does not give away ownership
        Arc::clone(&ptr)
    }

    fn store(&self, val: impl Into<Self::Elem>, order: Ordering) {
        let ptr: Self::Elem = val.into();
        let new_data = Arc::into_raw(ptr) as usize;

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .store(new_data, order)
        }
    }

    fn swap(&self, val: impl Into<Self::Elem>, order: Ordering) -> Self::Elem {
        let ptr: Self::Elem = val.into();
        let new_data = Arc::into_raw(ptr) as usize;

        unsafe {
            let old_data = transmute::<&Self, &AtomicUsize>(self)
                .swap(new_data, order);
            Arc::from_raw(old_data as *const T)
        }
    }

    fn compare_exchange(&self, current: impl Into<Self::Elem>, new: impl Into<Self::Elem>, success: Ordering, failure: Ordering) -> Result<Self::Elem, Self::Elem> {
        let current: Self::Elem = current.into();
        let current = Arc::into_raw(current) as usize;
        let new: Self::Elem = new.into();
        let new = Arc::into_raw(new) as usize;

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    Arc::from_raw(ok as *const T)
                })
                .map_err(|err| {
                    Arc::from_raw(err as *const T)
                })
        }
    }

    fn compare_exchange_weak(&self, current: impl Into<Self::Elem>, new: impl Into<Self::Elem>, success: Ordering, failure: Ordering) -> Result<Self::Elem, Self::Elem> {
        let current: Self::Elem = current.into();
        let current = Arc::into_raw(current) as usize;
        let new: Self::Elem = new.into();
        let new = Arc::into_raw(new) as usize;

        unsafe {
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    Arc::from_raw(ok as *const T)
                })
                .map_err(|err| {
                    Arc::from_raw(err as *const T)
                })
        }
    }
}



#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use std::mem::size_of;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    #[test]
    fn size_of_option_arc() {
        let size = size_of::<Option<Arc<NonZeroUsize>>>();
        println!("{:?}", &size);
        assert_eq!(size, size_of::<usize>());
    }

    #[cfg(feature = "tag")]
    #[test]
    fn size_of_option_tagged() {
        let size = size_of::<Option<TaggedArc<String>>>();
        println!("{:?}", &size);
        assert_eq!(size, size_of::<usize>());
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Wrapper {
        pub inner: NonZeroUsize
    }

    struct ArcWrapper<T>(Arc<T>);

    #[test]
    fn size_of_option_wrapper_arc() {
        let size = size_of::<ArcWrapper<&str>>();
        println!("{:?}", size);
    }
    
    #[test]
    fn test_transmute_ref_option() {
        let opt = Some(Wrapper {
            inner: NonZeroUsize::new(13).unwrap()
        });
        println!("[1] opt was originally: {:?}", &opt);
        unsafe {
            transmute::<&Option<Wrapper>, &AtomicUsize>(&&opt)
                .swap(0, Ordering::Relaxed);
        }
        println!("[2] opt becomes {:?}", &opt);
        assert_eq!(opt, None);
    }

    #[test]
    fn test_transmute_copy_ref_option() {
        let opt = Some(Wrapper {
            inner: NonZeroUsize::new(13).unwrap()
        });
        println!("[1] opt was originally: {:?}", &opt);
        unsafe {
            let ret = transmute_copy::<Option<Wrapper>, AtomicUsize>(&opt)
                .load(Ordering::Relaxed);
            println!("[2] returned by load: {:?}", ret);
        }

        // unsafe {
        //     let ret = transmute::<&Option<Wrapper>, &AtomicUsize>(&opt)
        //         .swap(0, Ordering::Relaxed);
        //     println!("[2] returned by swap: {:?}", ret);
        // }
        println!("[3] opt becomes {:?}", opt);
        // assert_eq!(opt, None);
    }
}