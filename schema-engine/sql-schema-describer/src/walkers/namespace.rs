use crate::{NamespaceId, Walker};

/// Traverse a namespace
pub type NamespaceWalker<'a> = Walker<'a, NamespaceId>;

impl<'a> NamespaceWalker<'a> {
    /// The namespace name.
    pub fn name(self) -> &'a str {
        &self.schema.namespaces[self.id.0 as usize]
    }
}
