pub(crate) mod fields;
pub(crate) mod objects;

use super::*;
use fields::*;
use itertools::Itertools;
use prisma_models::ScalarFieldRef;
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
        TypeIdentifier::Enum(e) => map_enum_input_type(ctx, e),
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

fn map_enum_input_type(ctx: &mut BuilderContext, enum_name: &str) -> InputType {
    let e = ctx
        .internal_data_model
        .find_enum(enum_name)
        .expect("Enum references must always be valid.");

    let et: EnumType = e.into();

    et.into()
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

fn compound_object_name(alias: Option<&String>, from_fields: &[(Vec<String>, ScalarFieldRef)]) -> String {
    alias.map(capitalize).unwrap_or_else(|| {
        let field_names: Vec<String> = from_fields
            .iter()
            .map(|(path, field)| path.iter().map(capitalize).join(""))
            .collect();

        field_names.join("")
    })
}

fn model_field_enum(model: &ModelRef) -> EnumTypeRef {
    Arc::new(EnumType::FieldRef(FieldRefEnumType {
        name: format!("{}ScalarFieldEnum", capitalize(&model.name)),
        values: model
            .fields()
            .scalar()
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect(),
    }))
}
