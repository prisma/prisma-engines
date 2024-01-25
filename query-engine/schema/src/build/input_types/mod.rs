pub(crate) mod fields;
pub(crate) mod objects;

use super::*;
use fields::*;
use query_structure::ScalarFieldRef;

fn map_scalar_input_type_for_field<'a>(ctx: &'a QuerySchema, field: &ScalarFieldRef) -> InputType<'a> {
    map_scalar_input_type(ctx, field.type_identifier(), field.is_list())
}

fn map_scalar_input_type(ctx: &'_ QuerySchema, typ: TypeIdentifier, list: bool) -> InputType<'_> {
    let typ = match typ {
        TypeIdentifier::String => InputType::string(),
        TypeIdentifier::Int => InputType::int(),
        TypeIdentifier::Float => InputType::float(),
        TypeIdentifier::Decimal => InputType::decimal(),
        TypeIdentifier::Boolean => InputType::boolean(),
        TypeIdentifier::UUID => InputType::uuid(),
        TypeIdentifier::DateTime => InputType::date_time(),
        TypeIdentifier::Json => InputType::json(),
        TypeIdentifier::Enum(id) => InputType::enum_type(map_schema_enum_type(ctx, id)),
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
pub(crate) fn list_union_object_type(input: InputObjectType<'_>, as_list: bool) -> Vec<InputType<'_>> {
    let input_type = InputType::object(input);
    list_union_type(input_type, as_list)
}

/// Convenience function to return [input_type, list_input_type]
/// (shorthand + full type) if the field is a list.
pub(crate) fn list_union_type(input_type: InputType<'_>, as_list: bool) -> Vec<InputType<'_>> {
    if as_list {
        vec![input_type.clone(), InputType::list(input_type)]
    } else {
        vec![input_type]
    }
}

fn compound_object_name(alias: Option<&str>, from_fields: &[ScalarFieldRef]) -> String {
    alias.map(|a| capitalize(a).to_string()).unwrap_or_else(|| {
        let field_names: Vec<String> = from_fields
            .iter()
            .map(|field| capitalize(field.name()).to_string())
            .collect();
        field_names.join("")
    })
}
