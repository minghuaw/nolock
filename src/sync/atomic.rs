use std::sync::atomic::Ordering;

pub trait Atomic {
    type Elem;

    /// Loads a value from the atomic pointer.
    ///
    /// `load` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Acquire` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Release` or `AcqRel`.
    fn load(&self, order: Ordering) -> Self::Elem;
    
    /// Stores a value into the pointer
    ///
    /// `store` takes an `Ordering` argument which describes 
    /// the memory ordering of this operation. 
    /// Possible values are `SeqCst`, `Release` and `Relaxed`.
    ///
    /// # Panics
    /// 
    /// Panics if `order` is `Acquire` or `AcqRel`.
    fn store(&self, val: impl Into<Self::Elem>, order: Ordering);
    
    /// Stores a `TaggedArc` pointer into the atomic pointer, returning the previously stored pointer
    ///
    /// swap takes an `Ordering` argument which describes the memory ordering of this operation. 
    /// All ordering modes are possible. Note that using `Acquire` makes the store part of this 
    /// operation `Relaxed`, and using `Release` makes the load part `Relaxed`.
    fn swap(&self, val: impl Into<Self::Elem>, order: Ordering) -> Self::Elem;

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
        current: impl Into<Self::Elem>,
        new: impl Into<Self::Elem>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Elem, Self::Elem>;

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
        current: impl Into<Self::Elem>,
        new: impl Into<Self::Elem>,
        success: Ordering,
        failure: Ordering
    ) -> Result<Self::Elem, Self::Elem>;

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
    fn fetch_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        mut f: F 
    ) -> Result<Self::Elem, Self::Elem>
    where 
        F: FnMut(&Self::Elem) -> Option<Self::Elem>
    {
        let mut prev = self.load(fetch_order);
        while let Some(next) = f(&prev) {
            match self.compare_exchange_weak(prev, next, set_order, fetch_order) {
                x @ Ok(_) => return x,
                Err(next_prev) => prev = next_prev
            }
        }
        Err(prev)
    }
}
