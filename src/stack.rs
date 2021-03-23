use std::sync::atomic::AtomicPtr;

struct Node<T> {
    data: T,
    next: Option<AtomicPtr<Node<T>>>
}