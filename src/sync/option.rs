
#[cfg(feature = "tag")]
use super::TaggedArc;

pub enum AtomicOption<T> {
    Some(T),
    None,
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
        
    }
}