use std::num::NonZeroUsize;

#[cfg(feature = "tag")]
use super::TaggedArc;

pub enum AtomicOption<T> {
    Some(T),
    None,
}

