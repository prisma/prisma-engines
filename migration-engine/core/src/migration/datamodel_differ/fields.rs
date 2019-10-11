use super::directives::directives_match_exactly;
use datamodel::ast;

/// Implements the logic to diff a pair of [Field ASTs](/datamodel/ast/struct.Field.html).
pub(crate) struct FieldDiffer<'a> {
    pub(crate) previous: &'a ast::Field,
    pub(crate) next: &'a ast::Field,
}

impl<'a> FieldDiffer<'a> {
    /// Has the type of the field changed? This ignores the arity and nullability of the field.
    pub(crate) fn type_changed(&self) -> bool {
        self.previous.field_type.name != self.next.field_type.name
    }

    // /// Has the nullability of the field changed?.
    // pub(crate) fn nullability_changed(&self) -> bool {
    //     match (&self.previous.arity, &self.next.arity) {
    //         (ast::FieldArity::Optional, ast::FieldArity::Required)
    //         | (ast::FieldArity::Required, ast::FieldArity::Optional) => true,
    //         _ => false,
    //     }
    // }

    // /// Has the arity of the field changed (list vs scalar)?.
    // pub(crate) fn arity_changed(&self) -> bool {
    //     match (&self.previous.arity, &self.next.arity) {
    //         (ast::FieldArity::List, ast::FieldArity::Optional)
    //         | (ast::FieldArity::List, ast::FieldArity::Required)
    //         | (ast::FieldArity::Optional, ast::FieldArity::List)
    //         | (ast::FieldArity::Required, ast::FieldArity::List) => true,
    //         _ => false,
    //     }
    // }

    fn previous_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    fn next_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    /// Iterator over the directives present in `next` but not in `previous`.
    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next_directive| {
            self.previous_directives()
                .find(|previous_directive| directives_match_exactly(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the directives present in `previous` but not in `next`.
    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| directives_match_exactly(previous_directive, next_directive))
                .is_none()
        })
    }
}
