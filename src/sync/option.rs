use std::{intrinsics::transmute, mem::transmute_copy, num::NonZeroUsize, sync::{Arc, atomic::AtomicUsize}, usize};
use std::sync::atomic::Ordering;

use super::Atomic;

#[cfg(feature = "tag")]
use super::TaggedArc;

#[cfg(feature = "tag")]
impl<T> Atomic for Option<TaggedArc<T>> {
    type Target = Self;

    fn load(&self, order: Ordering) -> Self {
        let ptr = unsafe {
            let addr = transmute_copy::<Self, AtomicUsize>(self) 
                .load(order);
            match TaggedArc::from_usize(addr) {
                Some(ptr) => ptr,
                None => return None
            }
        };
        // clone because `load` does not give away ownership
        Some(TaggedArc::clone(&ptr))
    }

    fn store(&self, new: impl Into<Self>, order: Ordering) {
        let new: Self = new.into();
        
        unsafe {
            let new_data = transmute::<Self, usize>(new);
            transmute::<&Self, &AtomicUsize>(self)
                .store(new_data, order)
        }
    }

    fn swap(&self, new: impl Into<Self::Target>, order: Ordering) -> Self::Target {
        let new: Self::Target = new.into();
        
        unsafe {
            let new_data = transmute::<Self, usize>(new);
            let old_data = transmute::<&Self, &AtomicUsize>(self)
                .swap(new_data, order);
            TaggedArc::from_usize(old_data) 
        }
    }

    fn compare_exchange(&self, current: impl Into<Self::Target>, new: impl Into<Self::Target>, success: Ordering, failure: Ordering) -> Result<Self::Target, Self::Target> {
        let current: Self::Target = current.into();
        let new: Self::Target = new.into();

        unsafe {
            let current = transmute::<Self, usize>(current);
            let new = transmute::<Self, usize>(new);
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    TaggedArc::from_usize(ok)
                })
                .map_err(|err| {
                    TaggedArc::from_usize(err)
                })
        }
    }

    fn compare_exchange_weak(&self, current: impl Into<Self::Target>, new: impl Into<Self::Target>, success: Ordering, failure: Ordering) -> Result<Self::Target, Self::Target> {
        let current: Self::Target = current.into();
        let new: Self::Target = new.into();

        unsafe {
            let current = transmute::<Self, usize>(current);
            let new = transmute::<Self, usize>(new);
            println!("current: 0x{:x}", current);
            println!("new: 0x{:x}", new);
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    println!("ok: 0x{:x}", ok);
                    TaggedArc::from_usize(ok).clone()
                })
                .map_err(|err| {
                    println!("err: 0x{:x}", err);
                    TaggedArc::from_usize(err).clone()
                })
        }
    }
}

impl<T> Atomic for Option<Arc<T>> {
    type Target = Self;

    fn load(&self, order: Ordering) -> Self::Target {
        let addr = unsafe { transmute_copy::<Self, AtomicUsize>(self) }
            .load(order);
        let ptr = match NonZeroUsize::new(addr) {
            Some(data) => {
                unsafe {
                    let data: usize = transmute(data);
                    Arc::from_raw(data as *const T)
                }
            },
            None => return None
        };
        // clone because `load` does not give away ownership
        Some(Arc::clone(&ptr))
    }

    fn store(&self, new: impl Into<Self::Target>, order: Ordering) {
        let new: Self::Target = new.into();

        unsafe {
            let new_data = transmute::<Self, usize>(new);
            transmute::<&Self, &AtomicUsize>(self)
                .store(new_data, order)
        }
    }

    fn swap(&self, new: impl Into<Self::Target>, order: Ordering) -> Self::Target {
        let new: Self::Target = new.into();

        unsafe {
            let new_data = transmute::<Self, usize>(new);
            let old_data = transmute::<&Self, &AtomicUsize>(self)
                .swap(new_data, order);
            match NonZeroUsize::new(old_data) {
                Some(data) => {
                    let data: usize = transmute(data);
                    Some(Arc::from_raw(data as *const T))
                },
                None => None
            }
        }
    }

    fn compare_exchange(&self, current: impl Into<Self::Target>, new: impl Into<Self::Target>, success: Ordering, failure: Ordering) -> Result<Self::Target, Self::Target> {
        let current: Self::Target = current.into();
        let new: Self::Target = new.into();

        unsafe {
            let current = transmute::<Self, usize>(current);
            let new = transmute::<Self, usize>(new);
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    match NonZeroUsize::new(ok) {
                        Some(data) => {
                            let data: usize = transmute(data);
                            Some(Arc::from_raw(data as *const T))
                        },
                        None => None
                    }
                })
                .map_err(|err| {
                    match NonZeroUsize::new(err) {
                        Some(data) => {
                            let data: usize = transmute(data);
                            Some(Arc::from_raw(data as *const T))
                        },
                        None => None
                    }
                })
        }
    }

    fn compare_exchange_weak(&self, current: impl Into<Self::Target>, new: impl Into<Self::Target>, success: Ordering, failure: Ordering) -> Result<Self, Self> {
        let current: Self::Target = current.into();
        let new: Self::Target = new.into();

        unsafe {
            let current = transmute::<Self, usize>(current);
            let new = transmute::<Self, usize>(new);
            transmute::<&Self, &AtomicUsize>(self)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    match NonZeroUsize::new(ok) {
                        Some(data) => {
                            let data: usize = transmute(data);
                            Some(Arc::from_raw(data as *const T))
                        },
                        None => None
                    }
                })
                .map_err(|err| {
                    match NonZeroUsize::new(err) {
                        Some(data) => {
                            let data: usize = transmute(data);
                            Some(Arc::from_raw(data as *const T))
                        },
                        None => None
                    }
                })
        }
    }
}



#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use std::{mem::size_of};
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

    #[test]
    fn test_store_to_none() {
        let opt: Option<TaggedArc<i32>> = None;
        assert_eq!(opt.is_none(), true);

        let ptr = TaggedArc::compose(Arc::new(13), 0);
        opt.store(ptr, Ordering::Relaxed);
        assert_eq!(opt.is_none(), false);
    }

    #[test]
    fn test_swap() {
        let opt = Some(TaggedArc::compose(Arc::new(13), 0));
        assert_eq!(opt.is_none(), false);

        opt.swap(None, Ordering::Relaxed);
        assert_eq!(opt.is_none(), true);
    }

    #[test]
    fn test_load() {
        let o = Some(TaggedArc::compose(Arc::new(13), 0));
        let out = o.load(Ordering::Relaxed);
        assert_eq!(out.is_none(), false);

        let o: Option<TaggedArc<i32>> = None;
        let out = o.load(Ordering::Relaxed);
        assert_eq!(out.is_none(), true);
    }

    #[test]
    fn test_() {
        unimplemented!();
    }

    #[test]
    fn test_taggedarc_compare_exchange_weak() {
        let arc = Arc::new(13);
        // let keep_a_copy = arc.clone();
        println!("[1] arc: {:p}", arc);
        let ptr = TaggedArc::compose(arc, 0);
        let old = Some(ptr);
        // println!("[2] old: {:p}", &old);

        let new_arc = Arc::new(15);
        println!("[3] new_arc: {:p}", new_arc);
        let new_ptr = TaggedArc::compose(new_arc, 0);
        let new = Some(new_ptr);
        // println!("[4] new: {:p}", &new);

        println!("-------------------");
        let current = old.load(Ordering::Relaxed);
        // let out = old.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire);
        let out = old.compare_exchange_weak(None, new, Ordering::Acquire, Ordering::Relaxed);
        // println!("{:?}", &out);
        // assert!(out.is_err());

        // let out = unsafe {
        //     let current: usize = transmute(current);
        //     let new: usize = transmute(new);
        //     let tmp = transmute::<&_, &AtomicUsize>(&old);    
        //     tmp.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire)        
        // };

        println!("-------------------");
        let current = old.load(Ordering::Relaxed);
        
        let new_arc = Arc::new(15);
        println!("[5] new_arc: {:p}", new_arc);
        let new_ptr = TaggedArc::compose(new_arc, 0);
        let new = Some(new_ptr);        
        
        // let new_arc = Arc::new(15);
        // println!("[5] new_arc: {:p}", new_arc);
        // let new_ptr = TaggedArc::compose(new_arc, 0);
        // let new = Some(new_ptr); 

        // let out = old.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire);
        let out = old.compare_exchange_weak(None, new, Ordering::AcqRel, Ordering::Acquire);
        println!("{:?}", out);
        // assert!(out.is_ok());

        // println!("-------------------");
        // let out = old.compare_exchange_weak(None, None, Ordering::Acquire, Ordering::Relaxed);
        // println!("{:?}", out);
    }    

    #[test]
    fn test_atomic_usize_compare_exchange_weak() {
        let a = AtomicUsize::new(3);
        let out = a.compare_exchange_weak(1, 4, Ordering::AcqRel, Ordering::Acquire);
        println!("{:?}", out);

        let out = a.compare_exchange_weak(3, 4, Ordering::AcqRel, Ordering::Acquire);
        println!("{:?}", out);
    }

    #[test]
    fn test_arc_compare_exchange_weak() {
        let old_arc = Arc::new(13);
        println!("[1] old arc: {:p}", old_arc);
        // let old = Some(old_arc);

        unsafe {
            // let data = transmute::<Arc<i32>, usize>(old_arc)
                // .load(Ordering::Acquire);
                // .into_inner();
                // ;

            let raw = Arc::into_raw(old_arc.clone());
            println!("{:p}", raw);
            let data = raw as usize;
            let data: AtomicUsize = transmute(data);
            let out = data.load(Ordering::Acquire);
            println!("data: 0x{:x}", out);
        }
    }

    #[test]
    fn test_arc_load() {
        let ptr = Arc::new(13);
        let ptr_addr = format!("{:p}", ptr);
        println!("ptr_addr: {}", ptr_addr);
        let opt = Some(ptr);
        println!("{:p}", &opt);
        // let out = opt.load(Ordering::Acquire); // this will give wrong result
        // This will create memory error
        // let out = unsafe {
        //     transmute::<_, AtomicUsize>(opt).load(Ordering::Acquire)
        // };

        let out = unsafe {
            opt.map(
                |a| {
                    let raw = Arc::into_raw(a);
                    let data = raw as usize;
                    transmute::<_, AtomicUsize>(data).load(Ordering::Acquire)
                }
            ).unwrap()
        };
        
        // match out {
        //     Some(a) => println!("{:p}", a),
        //     None => {}
        // }

        let out_addr = format!("0x{:x}", out);
        println!("out_addr: {}", out_addr);
        assert_eq!(ptr_addr, out_addr);
    }
}