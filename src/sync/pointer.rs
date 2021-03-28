use std::{mem::transmute, mem::transmute_copy, ptr::NonNull, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use std::marker::PhantomData;
use std::num::NonZeroUsize;

use super::{Atomic};

#[cfg(feature = "tag")]
use super::TaggedArc;

/// A wrapper that change all API to only accept and return `Arc` and allows tagging
///
/// If `feature = "tag"` is enabled, the tag will be stored in the unused lower bits 
/// of the pointer address.
pub struct AtomicArc<T> {
    // data is a usize that contains a pointer and a tag if `feature = "tag"`is enabled. 
    // The tag resides on the unused lower bits.
    data: NonNull<T>,
}

unsafe impl<T: Sync + Send> Send for AtomicArc<T> {}
unsafe impl<T: Sync + Send> Sync for AtomicArc<T> {}

impl<T> AtomicArc<T> {
    pub fn new<P: Into<Arc<T>>>(val: P) -> Self {
        let ptr: Arc<T> = val.into();
        Self::from_arc(ptr)
    }

    pub fn from_arc(val: Arc<T>) -> Self {
        let raw = Arc::into_raw(val) as *mut T;
        let data = unsafe { NonNull::new_unchecked(raw)};
        Self {
            data,
        }
    }

    #[cfg(feature = "tag")]
    pub fn from_tagged(tagged: TaggedArc<T>) -> Self {
        let data = tagged.data;
        Self {
            data,
        }
    }

    // Only API that expose Arc should be public
    pub unsafe fn from_usize(val: usize) -> Option<Self> {
        let data = NonZeroUsize::new(val)?;
        let ret = Self {
            data: unsafe { transmute(data) }
        };
        Some(ret)
    }

    pub fn get_mut() {
        unimplemented!()
    }
}

#[cfg(feature = "tag")]
impl<T> Atomic for AtomicArc<T> {
    type Elem = TaggedArc<T>;

    /// Loads a value from the atomic pointer.
    ///
    /// `load` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Acquire` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Release` or `AcqRel`.
    fn load(&self, order: Ordering) -> TaggedArc<T> {
        let ptr = unsafe {
            let addr = transmute_copy::<NonNull<T>, AtomicUsize>(&self.data)
                .load(order);
            TaggedArc::from_usize(addr)
                .expect("AtomicArc pointer must be non-zero")
        };
        // clone because `load` does not give away ownership
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
    fn store(&self, val: impl Into<TaggedArc<T>>, order: Ordering) {
        let ptr: TaggedArc<T> = val.into();
        let new_data = ptr.into_usize();
        // self.data.store(new_data, order)
        unsafe {
            transmute::<&NonNull<T>, &AtomicUsize>(&self.data)
                .store(new_data, order)
        }
    }

    /// Stores a `TaggedArc` pointer into the atomic pointer, returning the previously stored pointer
    ///
    /// swap takes an `Ordering` argument which describes the memory ordering of this operation. 
    /// All ordering modes are possible. Note that using `Acquire` makes the store part of this 
    /// operation `Relaxed`, and using `Release` makes the load part `Relaxed`.            
    fn swap(&self, val: impl Into<TaggedArc<T>>, order: Ordering) -> TaggedArc<T> {
        let ptr: TaggedArc<T> = val.into();
        let new_data = ptr.into_usize();
        // let old_data = self.data.swap(new_data, order);
        
        // SAFETY: only raw Arc pointers will be stored in the pointer
        unsafe {
            let old_data = transmute::<&NonNull<T>, &AtomicUsize>(&self.data)
                .swap(new_data, order);
            TaggedArc::from_usize(old_data)
                .expect("AtomicArc pointer must be non-zero")
        }
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
    fn compare_exchange(
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

        // SAFETY: The stored address must come from a valid Arc pointer
        unsafe {
            transmute::<&NonNull<T>, &AtomicUsize>(&self.data)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    TaggedArc::from_usize(ok)
                        .expect("AtomicArc pointer must be non-zero")
                })
                .map_err(|err| {
                    TaggedArc::from_usize(err)
                        .expect("AtomicArc pointer must be non-zero")
                })
        }
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
    fn compare_exchange_weak(
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
        // self.data.compare_exchange_weak(current, new, success, failure)
        //     .map(|success| {
        //         unsafe{ TaggedArc::from_usize(success) }
        //     })
        //     .map_err(|failure| {
        //         unsafe{ TaggedArc::from_usize(failure) }
        //     })

        unsafe {
            transmute::<&NonNull<T>, &AtomicUsize>(&self.data)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    TaggedArc::from_usize(ok)
                        .expect("AtomicArc pointer must be non-zero")
                })
                .map_err(|err| {
                    TaggedArc::from_usize(err)
                        .expect("AtomicArc pointer must be non-zero")
                })
        }
    }

    // /// Fetches the value, and applies a function to it that returns an optional
    // /// new value. Returns a `Result` of `Ok(previous_value)` if the function
    // /// returned `Some(_)`, else `Err(previous_value)`.
    // ///
    // /// Note: This may call the function multiple times if the value has been
    // /// changed from other threads in the meantime, as long as the function
    // /// returns `Some(_)`, but the function will have been applied only once to
    // /// the stored value.
    // ///
    // /// Note: This does not protect the program from the ABA problem. 
    // ///
    // /// `fetch_update` takes two [`Ordering`] arguments to describe the memory
    // /// ordering of this operation. The first describes the required ordering for
    // /// when the operation finally succeeds while the second describes the
    // /// required ordering for loads. These correspond to the success and failure
    // /// orderings of [`AtomicPtr::compare_exchange`] respectively.
    // ///
    // /// Using [`Acquire`] as success ordering makes the store part of this
    // /// operation [`Relaxed`], and using [`Release`] makes the final successful
    // /// load [`Relaxed`]. The (failed) load ordering can only be [`SeqCst`],
    // /// [`Acquire`] or [`Relaxed`] and must be equivalent to or weaker than the
    // /// success ordering.
    // fn fetch_update<F>(
    //     &self,
    //     set_order: Ordering,
    //     fetch_order: Ordering,
    //     mut f: F
    // ) -> Result<TaggedArc<T>, TaggedArc<T>>
    // where 
    //     F: FnMut(&TaggedArc<T>) -> Option<TaggedArc<T>>
    // {
    //     let mut prev = self.load(fetch_order);
    //     while let Some(next) = f(&prev) {
    //         match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
    //             x @ Ok(_) => return x,
    //             Err(next_prev) => prev = next_prev,
    //         }
    //     }
    //     Err(prev)
    // }
}

#[cfg(not(feature = "tag"))]
impl<T> Atomic for AtomicArc<T> {
    type Elem = Arc<T>;

    /// Loads a value from the atomic pointer.
    ///
    /// `load` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Acquire` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Release` or `AcqRel`.
    fn load(&self, order: Ordering) -> Arc<T> {
        let ptr = unsafe {
            let addr = transmute_copy::<NonZeroUsize, AtomicUsize>(&self.data)
                .load(order);
            Arc::from_raw(addr as *const T)
        };
        // clone because `load` doesn't give away ownership
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
    fn store(&self, val: impl Into<Arc<T>>, order: Ordering) {
        let ptr: Arc<T> = val.into();
        let new_data = Arc::into_raw(ptr) as usize;
        unsafe {
            transmute::<&NonZeroUsize, &AtomicUsize>(&self.data)
                .store(new_data, order)
        }
    }

    /// Stores a `Arc` pointer into the atomic pointer, returning the previously stored pointer
    ///
    /// swap takes an `Ordering` argument which describes the memory ordering of this operation. 
    /// All ordering modes are possible. Note that using `Acquire` makes the store part of this 
    /// operation `Relaxed`, and using `Release` makes the load part `Relaxed`.
    fn swap(&self, val: impl Into<Arc<T>>, order: Ordering) -> Arc<T> {
        let ptr: Arc<T> = val.into();
        let new_data = Arc::into_raw(ptr) as usize;
        // SAFETY: only raw Arc pointers will be stored in the pointer
        unsafe {
            let old_data = transmute::<&NonZeroUsize, &AtomicUsize>(&self.data)
                .swap(new_data, order);
            Arc::from_raw(old_data as *const T)
        }
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
    fn compare_exchange(
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

        unsafe {
            transmute::<&NonZeroUsize, &AtomicUsize>(&self.data)
                .compare_exchange(current, new, success, failure)
                .map(|ok| {
                    Arc::from_raw(ok as *const T)
                })
                .map_err(|err| {
                    Arc::from_raw(err as *const T)
                })
        }
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
    fn compare_exchange_weak(
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
        unsafe {
            transmute::<&NonZeroUsize, &AtomicUsize>(&self.data)
                .compare_exchange_weak(current, new, success, failure)
                .map(|ok| {
                    Arc::from_raw(ok as *const T)
                })
                .map_err(|err| {
                    Arc::from_raw(err as *const T)
                })
        }
    }

    // /// Fetches the value, and applies a function to it that returns an optional
    // /// new value. Returns a `Result` of `Ok(previous_value)` if the function
    // /// returned `Some(_)`, else `Err(previous_value)`.
    // ///
    // /// Note: This may call the function multiple times if the value has been
    // /// changed from other threads in the meantime, as long as the function
    // /// returns `Some(_)`, but the function will have been applied only once to
    // /// the stored value.
    // ///
    // /// Note: This does not protect the program from the ABA problem. 
    // ///
    // /// `fetch_update` takes two [`Ordering`] arguments to describe the memory
    // /// ordering of this operation. The first describes the required ordering for
    // /// when the operation finally succeeds while the second describes the
    // /// required ordering for loads. These correspond to the success and failure
    // /// orderings of [`AtomicPtr::compare_exchange`] respectively.
    // ///
    // /// Using [`Acquire`] as success ordering makes the store part of this
    // /// operation [`Relaxed`], and using [`Release`] makes the final successful
    // /// load [`Relaxed`]. The (failed) load ordering can only be [`SeqCst`],
    // /// [`Acquire`] or [`Relaxed`] and must be equivalent to or weaker than the
    // /// success ordering.
    // fn fetch_update<F>(
    //     &self,
    //     set_order: Ordering,
    //     fetch_order: Ordering,
    //     mut f: F
    // ) -> Result<Arc<T>, Arc<T>>
    // where 
    //     F: FnMut(&Arc<T>) -> Option<Arc<T>>
    // {
    //     let mut prev = self.load(fetch_order);
    //     while let Some(next) = f(&prev) {
    //         match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
    //             x @ Ok(_) => return x,
    //             Err(next_prev) => prev = next_prev,
    //         }
    //     }
    //     Err(prev)
    // }
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
    fn test_transmute_nonzerousize_to_atomicusize() {
        let nz = NonZeroUsize::new(13).unwrap();
        println!("[1] nz was originally: {:?}", &nz);
        unsafe {
            let ret = transmute_copy::<NonZeroUsize, AtomicUsize>(&nz)
                .swap(15, Ordering::Relaxed);
            println!("[2] returned by swap: {:?}", ret);
        }
        println!("[3] nz becomes: {:?}", nz);
    }
}