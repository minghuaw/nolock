use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use std::marker::PhantomData;


use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "tag")] {
        use std::mem;
        use std::borrow::Borrow;

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
        pub struct TaggedArc<T: ?Sized> {
            data: usize,
            _marker: PhantomData<T>
        }
        
        unsafe impl<T: ?Sized + Sync + Send> Send for TaggedArc<T> {}
        unsafe impl<T: ?Sized + Sync + Send> Sync for TaggedArc<T> {}
        
        impl<T> TaggedArc<T> {
            pub fn new(val: impl Into<Arc<T>>) -> Self {
                let ptr = val.into();
                Self::from_arc(ptr)
            }
        
            pub fn compose(val: impl Into<Arc<T>>, tag: usize) -> Self {
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
                let (ptr, _) = Self::decompose(self);
                ptr
            }
        
            pub fn decompose(ptr: impl Borrow<TaggedArc<T>>) -> (Arc<T>, usize) {
                let (data, tag) = decompose_tag::<T>(ptr.borrow().data);
                let ptr = data as *const T;
                unsafe {
                    (Arc::from_raw(ptr), tag)
                }
            }
        
            pub fn into_usize(self) -> usize {
                self.data
            }
        
            /// # Safety
            /// 
            /// `usize` may not be a valid pointer address
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
                // `compose_tag` will take care of removing any old tag
                // that is already with the current self.data
                let data = compose_tag::<T>(self.data, tag);
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
    }
}

/// A wrapper that change all API to only accept and return `Arc` and allows tagging
pub struct AtomicArc<T> {
    data: AtomicUsize,
    _marker: PhantomData<T>,
}

unsafe impl<T: Sync + Send> Send for AtomicArc<T> {}
unsafe impl<T: Sync + Send> Sync for AtomicArc<T> {}

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

    #[cfg(feature = "tag")]
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
}

cfg_if!{
    if #[cfg(feature = "tag")] {
        impl<T> AtomicArc<T> {
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
                let ptr = unsafe { TaggedArc::from_usize(data) };
                // clone because load does not give away ownership
                TaggedArc::clone(&ptr)
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

            /// Stores a `TaggedArc` pointer into the atomic pointer, returning the previously stored pointer
            ///
            /// swap takes an `Ordering` argument which describes the memory ordering of this operation. 
            /// All ordering modes are possible. Note that using `Acquire` makes the store part of this 
            /// operation `Relaxed`, and using `Release` makes the load part `Relaxed`.            
            pub fn swap<P: Into<TaggedArc<T>>>(&self, val: P, order: Ordering) -> TaggedArc<T> {
                let ptr: TaggedArc<T> = val.into();
                let new_data = ptr.into_usize();
                let old_data = self.data.swap(new_data, order);
                
                // SAFETY: only raw Arc pointers will be stored in the pointer
                unsafe { TaggedArc::from_usize(old_data) }
            }   

            /// Stores a `TaggedArc` pointer into the if the current value is the same as the `current` value.
            /// The tag will also be compared.
            ///
            /// The return value is a result indicating whether the new value was written and containing
            /// the previous value. On success this value is guaranteed to be equal to `current`.
            ///
            /// `compare_exchange` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. `success` describes the required ordering for the
            /// read-modify-write operation that takes place if the comparison with `current` succeeds.
            /// `failure` describes the required ordering for the load operation that takes place when
            /// the comparison fails. Using [`Acquire`] as success ordering makes the store part
            /// of this operation [`Relaxed`], and using [`Release`] makes the successful load
            /// [`Relaxed`]. The failure ordering can only be [`SeqCst`], [`Acquire`] or [`Relaxed`]
            /// and must be equivalent to or weaker than the success ordering.
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

                /// Stores an `Arc` pointer into the atomic pointer if the current value is the same as the `current` value.
            ///
            /// Unlike [`compare_exchange`], this function is allowed to spuriously fail even when the
            /// comparison succeeds, which can result in more efficient code on some platforms. The
            /// return value is a result indicating whether the new value was written and containing the
            /// previous value.
            ///
            /// `compare_exchange_weak` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. `success` describes the required ordering for the
            /// read-modify-write operation that takes place if the comparison with `current` succeeds.
            /// `failure` describes the required ordering for the load operation that takes place when
            /// the comparison fails. Using [`Acquire`] as success ordering makes the store part
            /// of this operation [`Relaxed`], and using [`Release`] makes the successful load
            /// [`Relaxed`]. The failure ordering can only be [`SeqCst`], [`Acquire`] or [`Relaxed`]
            /// and must be equivalent to or weaker than the success ordering.
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

            /// Fetches the value, and applies a function to it that returns an optional
            /// new value. Returns a `Result` of `Ok(previous_value)` if the function
            /// returned `Some(_)`, else `Err(previous_value)`.
            ///
            /// Note: This may call the function multiple times if the value has been
            /// changed from other threads in the meantime, as long as the function
            /// returns `Some(_)`, but the function will have been applied only once to
            /// the stored value.
            ///
            /// Note: This does not protect the program from the ABA problem. 
            ///
            /// `fetch_update` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. The first describes the required ordering for
            /// when the operation finally succeeds while the second describes the
            /// required ordering for loads. These correspond to the success and failure
            /// orderings of [`AtomicPtr::compare_exchange`] respectively.
            ///
            /// Using [`Acquire`] as success ordering makes the store part of this
            /// operation [`Relaxed`], and using [`Release`] makes the final successful
            /// load [`Relaxed`]. The (failed) load ordering can only be [`SeqCst`],
            /// [`Acquire`] or [`Relaxed`] and must be equivalent to or weaker than the
            /// success ordering.
            pub fn fetch_update<F>(
                &self,
                set_order: Ordering,
                fetch_order: Ordering,
                mut f: F
            ) -> Result<TaggedArc<T>, TaggedArc<T>>
            where 
                F: FnMut(&TaggedArc<T>) -> Option<TaggedArc<T>>
            {
                let mut prev = self.load(fetch_order);
                while let Some(next) = f(&prev) {
                    match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
                        x @ Ok(_) => return x,
                        Err(next_prev) => prev = next_prev,
                    }
                }
                Err(prev)
            }
        }
     } else {
        impl<T> AtomicArc<T> {
            /// Loads a value from the atomic pointer.
            ///
            /// `load` takes an `Ordering` argument which describes 
            /// the memory ordering of this operation. 
            /// Possible values are `SeqCst`, `Acquire` and `Relaxed`.
            ///
            /// # Panics
            /// 
            /// Panics if `order` is `Release` or `AcqRel`.
            pub fn load(&self, order: Ordering) -> Arc<T> {
                let data = self.data.load(order);
                let raw = data as *const T;
                let ptr = unsafe { Arc::from_raw(raw) };
                // clone because load doesn't give away ownership
                Arc::clone(&ptr)
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
            pub fn store<P: Into<Arc<T>>>(&self, val: P, order: Ordering) {
                let ptr: Arc<T> = val.into();
                let new_data = Arc::into_raw(ptr) as usize;
                self.data.store(new_data, order)
            }

            /// Stores a `Arc` pointer into the atomic pointer, returning the previously stored pointer
            ///
            /// swap takes an `Ordering` argument which describes the memory ordering of this operation. 
            /// All ordering modes are possible. Note that using `Acquire` makes the store part of this 
            /// operation `Relaxed`, and using `Release` makes the load part `Relaxed`.
            pub fn swap<P: Into<Arc<T>>>(&self, val: P, order: Ordering) -> Arc<T> {
                let ptr: Arc<T> = val.into();
                let new_data = Arc::into_raw(ptr) as usize;
                let old_data = self.data.swap(new_data, order);
                let raw = old_data as *const T;

                // SAFETY: only raw Arc pointers will be stored in the pointer
                unsafe { Arc::from_raw(raw) }
            }

            /// Stores a `Arc` pointer into the if the current value is the same as the `current` value.
            ///
            /// The return value is a result indicating whether the new value was written and containing
            /// the previous value. On success this value is guaranteed to be equal to `current`.
            ///
            /// `compare_exchange` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. `success` describes the required ordering for the
            /// read-modify-write operation that takes place if the comparison with `current` succeeds.
            /// `failure` describes the required ordering for the load operation that takes place when
            /// the comparison fails. Using [`Acquire`] as success ordering makes the store part
            /// of this operation [`Relaxed`], and using [`Release`] makes the successful load
            /// [`Relaxed`]. The failure ordering can only be [`SeqCst`], [`Acquire`] or [`Relaxed`]
            /// and must be equivalent to or weaker than the success ordering.
            pub fn compare_exchange(
                &self,
                current: impl Into<Arc<T>>,
                new: impl Into<Arc<T>>,
                success: Ordering,
                failure: Ordering,
            ) -> Result<Arc<T>, Arc<T>> {
                let current: Arc<T> = current.into();
                let current = Arc::into_raw(current) as usize;
                let new: Arc<T> = new.into();
                let new = Arc::into_raw(new) as usize;
                self.data.compare_exchange(current, new, success, failure)
                    .map(|success| {
                        let raw = success as *const T;
                        unsafe { Arc::from_raw(raw) }
                    })
                    .map_err(|failure| {
                        let raw = failure as *const T;
                        unsafe { Arc::from_raw(raw) }
                    })
            }

            /// Stores an `Arc` pointer into the atomic pointer if the current value is the same as the `current` value.
            ///
            /// Unlike [`compare_exchange`], this function is allowed to spuriously fail even when the
            /// comparison succeeds, which can result in more efficient code on some platforms. The
            /// return value is a result indicating whether the new value was written and containing the
            /// previous value.
            ///
            /// `compare_exchange_weak` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. `success` describes the required ordering for the
            /// read-modify-write operation that takes place if the comparison with `current` succeeds.
            /// `failure` describes the required ordering for the load operation that takes place when
            /// the comparison fails. Using [`Acquire`] as success ordering makes the store part
            /// of this operation [`Relaxed`], and using [`Release`] makes the successful load
            /// [`Relaxed`]. The failure ordering can only be [`SeqCst`], [`Acquire`] or [`Relaxed`]
            /// and must be equivalent to or weaker than the success ordering.
            pub fn compare_exchange_weak(
                &self,
                current: impl Into<Arc<T>>,
                new: impl Into<Arc<T>>,
                success: Ordering,
                failure: Ordering,
            ) -> Result<Arc<T>, Arc<T>> {
                let current: Arc<T> = current.into();
                let current = Arc::into_raw(current) as usize;
                let new: Arc<T> = new.into();
                let new = Arc::into_raw(new) as usize;
                self.data.compare_exchange_weak(current, new, success, failure)
                    .map(|success| {
                        let raw = success as *const T;
                        unsafe{ Arc::from_raw(raw) }
                    })
                    .map_err(|failure| {
                        let raw = failure as *const T;
                        unsafe{ Arc::from_raw(raw) }
                    })
            }

            /// Fetches the value, and applies a function to it that returns an optional
            /// new value. Returns a `Result` of `Ok(previous_value)` if the function
            /// returned `Some(_)`, else `Err(previous_value)`.
            ///
            /// Note: This may call the function multiple times if the value has been
            /// changed from other threads in the meantime, as long as the function
            /// returns `Some(_)`, but the function will have been applied only once to
            /// the stored value.
            ///
            /// Note: This does not protect the program from the ABA problem. 
            ///
            /// `fetch_update` takes two [`Ordering`] arguments to describe the memory
            /// ordering of this operation. The first describes the required ordering for
            /// when the operation finally succeeds while the second describes the
            /// required ordering for loads. These correspond to the success and failure
            /// orderings of [`AtomicPtr::compare_exchange`] respectively.
            ///
            /// Using [`Acquire`] as success ordering makes the store part of this
            /// operation [`Relaxed`], and using [`Release`] makes the final successful
            /// load [`Relaxed`]. The (failed) load ordering can only be [`SeqCst`],
            /// [`Acquire`] or [`Relaxed`] and must be equivalent to or weaker than the
            /// success ordering.
            pub fn fetch_update<F>(
                &self,
                set_order: Ordering,
                fetch_order: Ordering,
                mut f: F
            ) -> Result<Arc<T>, Arc<T>>
            where 
                F: FnMut(&Arc<T>) -> Option<Arc<T>>
            {
                let mut prev = self.load(fetch_order);
                while let Some(next) = f(&prev) {
                    match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
                        x @ Ok(_) => return x,
                        Err(next_prev) => prev = next_prev,
                    }
                }
                Err(prev)
            }
        }
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