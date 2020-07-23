mod create_input_objects;
mod filter_arguments;
mod filter_input_objects;
mod input_fields;
mod update_input_objects;

pub use create_input_objects::*;
pub use filter_arguments::*;
pub use filter_input_objects::*;
pub use input_fields::*;
pub use update_input_objects::*;

use super::*;
use crate::schema::*;
use prisma_models::{RelationFieldRef, ScalarFieldRef};

fn map_optional_input_type(ctx: &BuilderContext, field: &ScalarFieldRef) -> InputType {
    InputType::opt(map_required_input_type(ctx, field))
}

fn map_required_input_type(ctx: &BuilderContext, field: &ScalarFieldRef) -> InputType {
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

    let typ = if field.is_list { InputType::list(typ) } else { typ };
    let typ = if !field.is_required { InputType::null(typ) } else { typ };

    typ
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
