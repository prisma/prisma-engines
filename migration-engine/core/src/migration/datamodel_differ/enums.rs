use super::{directives::DirectiveDiffer, enum_values::EnumValueDiffer};
use datamodel::ast;

/// Implements the logic to diff a pair of [AST enums](/datamodel/ast/struct.Datamodel.html).
#[derive(Debug)]
pub(crate) struct EnumDiffer<'a> {
    pub(crate) previous: &'a ast::Enum,
    pub(crate) next: &'a ast::Enum,
}

impl<'a> EnumDiffer<'a> {
    pub(crate) fn value_pairs<'b>(&'b self) -> impl Iterator<Item = EnumValueDiffer<'a>> + 'b {
        self.previous_values().filter_map(move |previous_value| {
            self.next_values()
                .find(|next_value| values_match(previous_value, next_value))
                .map(|next_value| EnumValueDiffer {
                    previous: previous_value,
                    next: next_value,
                })
        })
    }

    /// Iterator over the values present in `next` but not `previous`.
    pub(crate) fn created_values(&self) -> impl Iterator<Item = &ast::EnumValue> {
        self.next_values().filter(move |next_value| {
            self.previous_values()
                .find(|previous_value| values_match(previous_value, next_value))
                .is_none()
        })
    }

    /// Iterator over the values present in `previous` but not `next`.
    pub(crate) fn deleted_values(&self) -> impl Iterator<Item = &ast::EnumValue> {
        self.previous_values().filter(move |previous_value| {
            self.next_values()
                .find(|next_value| values_match(previous_value, next_value))
                .is_none()
        })
    }

    /// Enum directives (`@@`) created in `next`.
    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next_directive| {
            self.previous_directives()
                .find(|previous_directive| enum_directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Enum directives (`@@`) deleted in `next`.
    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| enum_directives_match(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the enum directives (`@@`) present in both `previous` and `next`.
    pub(crate) fn directive_pairs(&'a self) -> impl Iterator<Item = DirectiveDiffer<'a>> {
        self.previous_directives().filter_map(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| enum_directives_match(previous_directive, next_directive))
                .map(|next_directive| DirectiveDiffer {
                    previous: previous_directive,
                    next: next_directive,
                })
        })
    }

    fn previous_values<'b>(&'b self) -> impl Iterator<Item = &'a ast::EnumValue> + 'b {
        self.previous.values.iter()
    }

    fn next_values<'b>(&'b self) -> impl Iterator<Item = &'a ast::EnumValue> + 'b {
        self.next.values.iter()
    }

    fn previous_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    fn next_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter()
    }
}

fn values_match(previous: &ast::EnumValue, next: &ast::EnumValue) -> bool {
    previous.name.name == next.name.name
}

fn enum_directives_match(previous: &ast::Directive, next: &ast::Directive) -> bool {
    previous.name.name == next.name.name
}

#[cfg(test)]
mod tests {
    use super::super::TopDiffer;
    use super::*;
    use datamodel::ast::parser::parse;

    #[test]
    fn datamodel_differ_enum_differ_works() {
        let previous = r#"
        enum BetterBoolean {
            True
            NearlyTrue
            ProbablyFalse
            DefinitelyFalse
        }
        "#;
        let previous = parse(previous).unwrap();
        let next = r#"
        enum BetterBoolean {
            True
            ProbablyFalse
            MostlyTrue
        }
        "#;
        let next = parse(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let enum_diff: EnumDiffer<'_> = differ.enum_pairs().next().unwrap();

        let created_values: Vec<&str> = enum_diff.created_values().map(|val| val.name.name.as_str()).collect();
        assert_eq!(created_values, &["MostlyTrue"]);

        let deleted_values: Vec<&str> = enum_diff.deleted_values().map(|val| val.name.name.as_str()).collect();
        assert_eq!(deleted_values, &["NearlyTrue", "DefinitelyFalse"],);
    }
}
