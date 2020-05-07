use super::*;

/// Generic extension also used by the FilterInputTypeBuilder.
pub trait InputBuilderExtensions {
    fn map_optional_input_type(&self, field: &ScalarFieldRef) -> InputType {
        InputType::opt(self.map_required_input_type(field))
    }

    fn map_required_input_type(&self, field: &ScalarFieldRef) -> InputType {
        let typ = match field.type_identifier {
            TypeIdentifier::String => InputType::string(),
            TypeIdentifier::Int => InputType::int(),
            TypeIdentifier::Float => InputType::float(),
            TypeIdentifier::Boolean => InputType::boolean(),
            TypeIdentifier::UUID => InputType::uuid(),
            TypeIdentifier::DateTime => InputType::date_time(),
            TypeIdentifier::Json => InputType::json(),
            TypeIdentifier::Enum(_) => self.map_enum_input_type(&field),
        };

        let typ = if field.is_list { InputType::list(typ) } else { typ };
        let typ = if !field.is_required { InputType::null(typ) } else { typ };

        typ
    }

    fn map_enum_input_type(&self, field: &ScalarFieldRef) -> InputType {
        let internal_enum = field
            .internal_enum
            .as_ref()
            .expect("A field with TypeIdentifier Enum must always have an enum.");

        let et: EnumType = internal_enum.clone().into();
        et.into()
    }
}
