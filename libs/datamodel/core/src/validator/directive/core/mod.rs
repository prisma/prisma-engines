use crate::dml;
use crate::validator::directive::DirectiveListValidator;

mod default;
mod embedded;
mod id;
mod map;
mod relation;
mod unique_and_index;
mod updated_at;
mod utils;

/// Returns a directive list validator containing all builtin field directives.
pub fn new_builtin_field_directives() -> DirectiveListValidator<dml::Field> {
    let mut validator = DirectiveListValidator::<dml::Field>::new();

    validator.add(Box::new(map::MapDirectiveValidatorForField {}));
    validator.add(Box::new(id::IdDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::FieldLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(default::DefaultDirectiveValidator {}));
    validator.add(Box::new(relation::RelationDirectiveValidator {}));
    validator.add(Box::new(updated_at::UpdatedAtDirectiveValidator {}));

    validator
}

/// Returns a directive list validator containing all builtin model directives.
pub fn new_builtin_model_directives() -> DirectiveListValidator<dml::Model> {
    let mut validator = DirectiveListValidator::<dml::Model>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));
    validator.add(Box::new(embedded::EmbeddedDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelIndexDirectiveValidator {}));
    validator.add(Box::new(id::ModelLevelIdDirectiveValidator {}));

    validator
}

/// Returns a directive list validator containing all builtin enum directives.
pub fn new_builtin_enum_directives() -> DirectiveListValidator<dml::Enum> {
    let mut validator = DirectiveListValidator::<dml::Enum>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));

    validator
}

/// Returns a directive list validator containing all builtin enum value directives.
pub fn new_builtin_enum_value_directives() -> DirectiveListValidator<dml::EnumValue> {
    let mut validator = DirectiveListValidator::<dml::EnumValue>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));

    validator
}
