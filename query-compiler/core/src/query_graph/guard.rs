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
        match self.content.take() {
            Some(c) => c,
            None => panic!("Logic error: Attempted to unset empty graph guard."),
        }
    }

    pub fn take(&mut self) -> Option<T> {
        self.content.take()
    }

    pub fn borrow(&self) -> Option<&T> {
        self.content.as_ref()
    }

    pub fn borrow_mut(&mut self) -> Option<&mut T> {
        self.content.as_mut()
    }

    pub fn into_inner(self) -> Option<T> {
        self.content
    }
}
