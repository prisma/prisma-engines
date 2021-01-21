pub(crate) mod arguments;
pub(crate) mod field_filter_types;
pub(crate) mod input_fields;

mod objects;

use super::*;
use crate::{constants::inputs::filters, schema::*};
use objects::*;
use prisma_models::{RelationFieldRef, ScalarFieldRef};

/// Builds "<Model>OrderByInput" object types.
pub(crate) fn order_by_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let enum_type = Arc::new(string_enum_type(
        "SortOrder",
        vec![filters::ASC.to_owned(), filters::DESC.to_owned()],
    ));
    let ident = Identifier::new(format!("{}OrderByInput", model.name), PRISMA_NAMESPACE);

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.allow_at_most_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = model
        .fields()
        .scalar()
        .iter()
        .map(|sf| input_field(sf.name.clone(), InputType::Enum(enum_type.clone()), None).optional())
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn map_scalar_input_type_for_field(ctx: &mut BuilderContext, field: &ScalarFieldRef) -> InputType {
    map_scalar_input_type(ctx, &field.type_identifier, field.is_list)
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

fn compound_object_name(alias: Option<&String>, from_fields: &[ScalarFieldRef]) -> String {
    alias.map(capitalize).unwrap_or_else(|| {
        let field_names: Vec<String> = from_fields.iter().map(|field| capitalize(&field.name)).collect();
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
