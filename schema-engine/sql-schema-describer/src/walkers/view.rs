use std::ops::Range;

use crate::{View, ViewColumnId, ViewColumnWalker, ViewId, Walker};

/// Traverse a view
pub type ViewWalker<'a> = Walker<'a, ViewId>;

impl<'a> ViewWalker<'a> {
    /// The name of the view
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// The SQL definition of the view
    pub fn definition(self) -> Option<&'a str> {
        self.get().definition.as_deref()
    }

    /// The namespace of the view
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get_index(self.get().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    /// Traverse the view's columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = ViewColumnWalker<'a>> {
        self.columns_range().map(move |idx| self.walk(ViewColumnId(idx as u32)))
    }

    /// Description (comment) of the view.
    pub fn description(self) -> Option<&'a str> {
        self.get().description.as_deref()
    }

    fn columns_range(self) -> Range<usize> {
        super::range_for_key(&self.schema.view_columns, self.id, |(tid, _)| *tid)
    }

    fn get(self) -> &'a View {
        &self.schema.views[self.id.0 as usize]
    }
}
