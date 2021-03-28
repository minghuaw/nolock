use std::num::NonZeroUsize;

use super::TaggedArc;

pub enum AtomicOption<T> {
    Some(T),
    None,
}

