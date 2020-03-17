use super::DirectiveDiffer;
use datamodel::ast;

pub(crate) struct EnumValueDiffer<'a> {
    pub(crate) previous: &'a ast::EnumValue,
    pub(crate) next: &'a ast::EnumValue,
}

impl<'a> EnumValueDiffer<'a> {
    pub(crate) fn directive_pairs<'b>(&'b self) -> impl Iterator<Item = DirectiveDiffer<'a>> + 'b {
        self.previous_directives().filter_map(move |previous| {
            self.next_directives()
                .find(|next| enum_value_directives_match(previous, next))
                .map(|next| DirectiveDiffer { previous, next })
        })
    }

    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next| {
            !self
                .previous_directives()
                .any(|previous| enum_value_directives_match(previous, next))
        })
    }

    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous| {
            !self
                .next_directives()
                .any(|next| enum_value_directives_match(previous, next))
        })
    }

    fn previous_directives<'b>(&'b self) -> impl Iterator<Item = &'a ast::Directive> + 'b {
        self.previous.directives.iter()
    }

    fn next_directives<'b>(&'b self) -> impl Iterator<Item = &'a ast::Directive> + 'b {
        self.next.directives.iter()
    }
}

fn enum_value_directives_match(previous: &ast::Directive, next: &ast::Directive) -> bool {
    previous.name.name == next.name.name
}
