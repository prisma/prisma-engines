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
    input_object.set_one_of(true);

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(name, input_object.clone());

    let fields = model
        .fields()
        .scalar()
        .iter()
        .map(|sf| {
            input_field(
                sf.name.clone(),
                InputType::opt(InputType::Enum(enum_type.clone())),
                None,
            )
        })
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn map_optional_input_type(field: &ScalarFieldRef) -> InputType {
    InputType::opt(map_required_input_type(field))
}

fn map_required_input_type(field: &ScalarFieldRef) -> InputType {
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

    match (field.is_list, field.is_required) {
        (true, _) => InputType::list(typ),
        (false, true) => typ,
        (false, false) => InputType::null(typ),
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

/// Wraps an input object type into an option list object type.
fn wrap_list_input_object_type(input: InputObjectTypeWeakRef, as_list: bool) -> InputType {
    if as_list {
        InputType::opt(InputType::list(InputType::object(input)))
    } else {
        InputType::opt(InputType::object(input))
    }
}

fn compound_object_name(alias: Option<&String>, from_fields: &[ScalarFieldRef]) -> String {
    alias.map(|n| capitalize(n)).unwrap_or_else(|| {
        let field_names: Vec<String> = from_fields.iter().map(|field| capitalize(&field.name)).collect();
        field_names.join("")
    })
}

fn wrap_opt_input_object(o: InputObjectTypeWeakRef) -> InputType {
    InputType::opt(InputType::object(o))
}
