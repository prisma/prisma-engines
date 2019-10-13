use super::{
    directives::{directives_match_exactly, DirectiveDiffer},
    FieldDiffer,
};
use datamodel::ast;

/// Implements the logic to diff a pair of [AST enums](/datamodel/ast/struct.Datamodel.html).
pub(crate) struct EnumDiffer<'a> {
    pub(crate) previous: &'a ast::Enum,
    pub(crate) next: &'a ast::Enum,
}

impl<'a> EnumDiffer<'a> {
    fn previous_values(&self) -> impl Iterator<Item = &ast::EnumValue> {
        self.previous.values.iter()
    }

    fn next_values(&self) -> impl Iterator<Item = &ast::EnumValue> {
        self.next.values.iter()
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

    /// Whether the enum values changed in `next`.
    pub(crate) fn values_changed(&self) -> bool {
        self.created_values().next().is_some() || self.deleted_values().next().is_some()
    }

    fn previous_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous.directives.iter()
    }

    fn next_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next.directives.iter()
    }

    /// Enum directives (`@@`) created in `next`.
    pub(crate) fn created_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.next_directives().filter(move |next_directive| {
            self.previous_directives()
                .find(|previous_directive| directives_match_exactly(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Enum directives (`@@`) deleted in `next`.
    pub(crate) fn deleted_directives(&self) -> impl Iterator<Item = &ast::Directive> {
        self.previous_directives().filter(move |previous_directive| {
            self.next_directives()
                .find(|next_directive| directives_match_exactly(previous_directive, next_directive))
                .is_none()
        })
    }

    /// Iterator over the enum directives (@@) present in both `previous` and `next`.
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
}

fn values_match(previous: &ast::EnumValue, next: &ast::EnumValue) -> bool {
    previous.name == next.name
}

#[cfg(test)]
mod tests {
    use super::super::TopDiffer;
    use super::*;
    use datamodel::parse_to_ast;

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
        let previous = parse_to_ast(previous).unwrap();
        let next = r#"
        enum BetterBoolean {
            True
            ProbablyFalse
            MostlyTrue
        }
        "#;
        let next = parse_to_ast(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let enum_diff: EnumDiffer<'_> = differ.enum_pairs().next().unwrap();

        let created_values: Vec<&str> = enum_diff.created_values().map(|val| val.name.as_str()).collect();
        assert_eq!(created_values, &["MostlyTrue"]);

        let deleted_values: Vec<&str> = enum_diff.deleted_values().map(|val| val.name.as_str()).collect();
        assert_eq!(deleted_values, &["NearlyTrue", "DefinitelyFalse"],);
    }
}

fn enum_directives_match(previous: &ast::Directive, next: &ast::Directive) -> bool {
    if previous.name.name != next.name.name {
        return false;
    }

    if ["unique", "index"].contains(&previous.name.name.as_str()) {
        // TODO: implement fine grained index diffing
        return false;
    }

    true
}
