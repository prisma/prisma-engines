use super::directive::{
    new_builtin_enum_directives, new_builtin_enum_value_directives, new_builtin_field_directives,
    new_builtin_model_directives, DirectiveListValidator,
};
use crate::dml;

pub struct DirectiveBox {
    pub field: DirectiveListValidator<dml::Field>,
    pub model: DirectiveListValidator<dml::Model>,
    pub enm: DirectiveListValidator<dml::Enum>,
    pub enm_value: DirectiveListValidator<dml::EnumValue>,
}

impl DirectiveBox {
    /// Creates a new instance, with all builtin directives registered.
    pub fn new() -> DirectiveBox {
        DirectiveBox {
            field: new_builtin_field_directives(),
            model: new_builtin_model_directives(),
            enm: new_builtin_enum_directives(),
            enm_value: new_builtin_enum_value_directives(),
        }
    }
}
