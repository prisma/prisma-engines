use super::{attributes::AttributeDiffer, enum_values::EnumValueDiffer};
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

    /// Enum attributes (`@@`) created in `next`.
    pub(crate) fn created_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next_attributes().filter(move |next_attribute| {
            self.previous_attributes()
                .find(|previous_attribute| enum_attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    /// Enum attributes (`@@`) deleted in `next`.
    pub(crate) fn deleted_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous_attributes().filter(move |previous_attribute| {
            self.next_attributes()
                .find(|next_attribute| enum_attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    /// Iterator over the enum attributes (`@@`) present in both `previous` and `next`.
    pub(crate) fn attribute_pairs(&'a self) -> impl Iterator<Item = AttributeDiffer<'a>> {
        self.previous_attributes().filter_map(move |previous_attribute| {
            self.next_attributes()
                .find(|next_attribute| enum_attributes_match(previous_attribute, next_attribute))
                .map(|next_attribute| AttributeDiffer {
                    previous: previous_attribute,
                    next: next_attribute,
                })
        })
    }

    fn previous_values<'b>(&'b self) -> impl Iterator<Item = &'a ast::EnumValue> + 'b {
        self.previous.values.iter()
    }

    fn next_values<'b>(&'b self) -> impl Iterator<Item = &'a ast::EnumValue> + 'b {
        self.next.values.iter()
    }

    fn previous_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous.attributes.iter()
    }

    fn next_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next.attributes.iter()
    }
}

fn values_match(previous: &ast::EnumValue, next: &ast::EnumValue) -> bool {
    previous.name.name == next.name.name
}

fn enum_attributes_match(previous: &ast::Attribute, next: &ast::Attribute) -> bool {
    previous.name.name == next.name.name
}

#[cfg(test)]
mod tests {
    use super::super::TopDiffer;
    use super::*;
    use datamodel::ast::parser::parse_schema;

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
        let previous = parse_schema(previous).unwrap();
        let next = r#"
        enum BetterBoolean {
            True
            ProbablyFalse
            MostlyTrue
        }
        "#;
        let next = parse_schema(next).unwrap();

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
