pub(crate) mod fields;
pub(crate) mod objects;

use super::*;
use crate::enum_types::*;
use fields::*;
use itertools::Itertools;
use prisma_models::{IndexField, ScalarFieldRef};
use schema::*;

fn map_scalar_input_type_for_field(ctx: &mut BuilderContext, field: &ScalarFieldRef) -> InputType {
    map_scalar_input_type(ctx, &field.type_identifier, field.is_list())
}

fn map_scalar_input_type(ctx: &mut BuilderContext, typ: &TypeIdentifier, list: bool) -> InputType {
    let typ = match typ {
        TypeIdentifier::String => InputType::string(),
        TypeIdentifier::Int => InputType::int(),
        TypeIdentifier::Float => InputType::float(),
        TypeIdentifier::Decimal => InputType::decimal(),
        TypeIdentifier::Boolean => InputType::boolean(),
        TypeIdentifier::UUID => InputType::uuid(),
        TypeIdentifier::DateTime => InputType::date_time(),
        TypeIdentifier::Json => InputType::json(),
        TypeIdentifier::Enum(e) => InputType::enum_type(map_schema_enum_type(ctx, e)),
        TypeIdentifier::Xml => InputType::xml(),
        TypeIdentifier::Bytes => InputType::bytes(),
        TypeIdentifier::BigInt => InputType::bigint(),
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    };

    if list {
        InputType::list(typ)
    } else {
        typ
    }
}

/// Convenience function to return [object_type, list_object_type]
/// (shorthand + full type) if the field is a list.
fn list_union_object_type(input: InputObjectTypeWeakRef, as_list: bool) -> Vec<InputType> {
    let input_type = InputType::object(input);
    list_union_type(input_type, as_list)
}

/// Convenience function to return [input_type, list_input_type]
/// (shorthand + full type) if the field is a list.
fn list_union_type(input_type: InputType, as_list: bool) -> Vec<InputType> {
    if as_list {
        vec![input_type.clone(), InputType::list(input_type)]
    } else {
        vec![input_type]
    }
}

fn compound_object_name(alias: Option<&String>, index_fields: &[IndexField]) -> String {
    alias.map(capitalize).unwrap_or_else(|| {
        let field_names: Vec<String> = index_fields
            .iter()
            .map(|index_field| index_field.path().iter().map(capitalize).join(""))
            .collect();

        field_names.join("")
    })
}
