use super::directives::{directives_match, DirectiveDiffer};
use datamodel::ast;

/// Implements the logic to diff a pair of [Field ASTs](/datamodel/ast/struct.Field.html).
#[derive(Debug)]
pub(crate) struct FieldDiffer<'a> {
    pub(crate) previous: &'a ast::Field,
    pub(crate) next: &'a ast::Field,
}

impl<'a> FieldDiffer<'a> {
    /// Iterator over the directives present in `next` but not in `previous`.
    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next_directive| {
            self.previous_directives()
                .find(|previous_directive| directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the directives present in `previous` but not in `next`.
    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    pub(crate) fn directive_pairs(&self) -> impl Iterator<Item = DirectiveDiffer> {
        self.previous_directives().filter_map(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| directives_match(previous_directive, next_directive))
                .map(|next_directive| DirectiveDiffer {
                    previous: previous_directive,
                    next: next_directive,
                })
        })
    }

    fn previous_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    fn next_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter()
    }
}
