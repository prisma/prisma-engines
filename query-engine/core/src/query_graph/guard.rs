use std::borrow::Borrow;

/// Guard struct to allow the QueryGraph implementation to retain
/// empty nodes and edges, instead of destroying parts of the graph
/// and losing information in the process.
/// Considered an implementation detail of the QueryGraph.
pub(super) struct Guard<T: Sized> {
    content: Option<T>,
}

impl<T> Guard<T> {
    pub fn new(content: T) -> Self {
        Guard { content: Some(content) }
    }

    pub fn unset(&mut self) -> T {
        let content = std::mem::replace(&mut self.content, None);
        match content {
            Some(c) => c,
            None => panic!("Logic error: Attempted to unset empty graph guard."),
        }
    }
}

impl<T> Borrow<T> for Guard<T> {
    fn borrow(&self) -> &T {
        match self.content {
            Some(ref c) => c,
            None => panic!("Logic error: Attempted to borrow empty graph guard."),
        }
    }
}
