use std::any::Any;

/// A data structure for soundly holding onto a list of objects with intrusive pointers between
/// them.
///
/// # Safety
/// There is no unsafe code within this module. The purpose of this safety section is to explain
/// what (typically unsafe) things you can do, *using* this module to keep your code sound.
///
/// **The type system says** that any item added with [`Self::push`] is `'static`, and does not hold
/// references to any temporary values that it may need to worry about outliving.
///
/// **We allow** any item added with [`Self::push`] to hold references to temporary values **if and
/// only if**:
/// - Those temporary values were returned by a call to [`Self::get`] on the same `UnsafeStore` that
///     the item is being pushed onto.
/// - Those temporary values were returned by a call to [`Self::get`] **before** we pushed that item
///     onto the `UnsafeStore` (or more specifically, that they were pushed onto the `UnsafeStore`
///     before this item).
///
/// # Invariants
/// 1. Items earlier in the list must outlive items later in the list.
/// 2. References handed out by [`Self::get`] must be stable (there can be no [`Self::get_mut`] API,
///     and we must ensure that the pointers we hand out remain valid even when the `Vec` resizes).
pub struct UnsafeStore {
    items: Vec<Box<dyn Any>>
}

impl UnsafeStore {
    pub fn new() -> Self {
        UnsafeStore {
            items: Vec::new(),
        }
    }

    pub fn get(store: &Self, item: usize) -> Option<&dyn Any> {
        // Invariant 2: we hand out a reference to the memory allocated by the Box itself, rather
        // than a reference to memory allocated by the Vec. This way, calls to push (which may
        // resize the vec) cannot invalidate our pointers.
        store.items.get(item).as_ref().map(|x| &***x)
    }

    pub fn push(store: &mut Self, item: Box<dyn Any>) -> usize {
        let output = store.items.len();
        store.items.push(item);
        output
    }
}

impl Drop for UnsafeStore {
    fn drop(&mut self) {
        // Invariant 1: make sure we drop in reverse order, or there is a brief window where
        while let Some(item) = self.items.pop() {
            drop(item)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    struct DropObserver {
        id: usize,
        sender: mpsc::Sender<usize>,
    }

    impl DropObserver {
        fn new(id: usize, sender: &mpsc::Sender<usize>) -> Box<dyn Any> {
            Box::new(DropObserver { id, sender: sender.clone() })
        }
    }

    impl Drop for DropObserver {
        fn drop(&mut self) {
            self.sender.send(self.id).unwrap()
        }
    }

    // invariant 1
    #[test]
    fn drop_in_reverse_order() {
        let (send, recv) = mpsc::channel();

        let mut store = UnsafeStore::new();
        UnsafeStore::push(&mut store, DropObserver::new(0, &send));
        UnsafeStore::push(&mut store, DropObserver::new(1, &send));
        UnsafeStore::push(&mut store, DropObserver::new(2, &send));
        UnsafeStore::push(&mut store, DropObserver::new(3, &send));
        UnsafeStore::push(&mut store, DropObserver::new(4, &send));
        drop(store);
        drop(send);

        assert_eq!(recv.recv(), Ok(4));
        assert_eq!(recv.recv(), Ok(3));
        assert_eq!(recv.recv(), Ok(2));
        assert_eq!(recv.recv(), Ok(1));
        assert_eq!(recv.recv(), Ok(0));
        assert_eq!(recv.recv(), Err(mpsc::RecvError));
    }

    // invariant 2, run this one in MIRI
    #[test]
    fn pointers_are_stable() {
        let mut store = UnsafeStore::new();
        let index = UnsafeStore::push(&mut store, Box::new(42i32));
        let reference = UnsafeStore::get(&store, index).unwrap();
        let ptr = &raw const *reference;

        for _ in 0..5000 {
            UnsafeStore::push(&mut store, Box::new("garbage"));
        }

        let reference = unsafe {
            &*ptr
        };
        assert_eq!(reference.downcast_ref::<i32>(), Some(&42i32));
    }
}