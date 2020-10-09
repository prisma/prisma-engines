pub(crate) mod create_input_objects;
pub(crate) mod field_filter_types;
pub(crate) mod filter_input_objects;
pub(crate) mod input_fields;
pub(crate) mod update_input_objects;

use super::*;
use crate::schema::*;
use prisma_models::{RelationFieldRef, ScalarFieldRef};

/// Builds "<Model>OrderByInput" object types.
pub(crate) fn order_by_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let enum_type = Arc::new(string_enum_type("SortOrder", vec!["asc".to_owned(), "desc".to_owned()]));
    let name = format!("{}OrderByInput", model.name);

    return_cached_input!(ctx, &name);

    let mut input_object = init_input_object_type(name.clone());
    input_object.allow_at_most_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(name, input_object.clone());

    let fields = model
        .fields()
        .scalar()
        .iter()
        .map(|sf| input_field(sf.name.clone(), InputType::Enum(enum_type.clone()), None).optional())
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn map_scalar_input_type(field: &ScalarFieldRef) -> InputType {
    let typ = match field.type_identifier {
        TypeIdentifier::String => InputType::string(),
        TypeIdentifier::Int => InputType::int(),
        TypeIdentifier::Float => InputType::float(),
        TypeIdentifier::Boolean => InputType::boolean(),
        TypeIdentifier::UUID => InputType::uuid(),
        TypeIdentifier::DateTime => InputType::date_time(),
        TypeIdentifier::Json => InputType::json(),
        TypeIdentifier::Enum(_) => map_enum_input_type(&field),
    };

    if field.is_list {
        InputType::list(typ)
    } else {
        typ
    }
}

fn map_enum_input_type(field: &ScalarFieldRef) -> InputType {
    let internal_enum = field
        .internal_enum
        .as_ref()
        .expect("A field with TypeIdentifier Enum must always have an associated internal enum.");

    let et: EnumType = internal_enum.clone().into();
    et.into()
}

/// Convenience function to return [object_type, list_object_type]
/// (shorthand + full type) if the field is a list.
fn list_union_type(input: InputObjectTypeWeakRef, as_list: bool) -> Vec<InputType> {
    let object_type = InputType::object(input);

    if as_list {
        vec![object_type.clone(), InputType::list(object_type)]
    } else {
        vec![object_type]
    }
}

fn compound_object_name(alias: Option<&String>, from_fields: &[ScalarFieldRef]) -> String {
    alias.map(capitalize).unwrap_or_else(|| {
        let field_names: Vec<String> = from_fields.iter().map(|field| capitalize(&field.name)).collect();
        field_names.join("")
    })
}
