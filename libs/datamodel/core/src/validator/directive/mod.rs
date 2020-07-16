mod default;
mod directive_list_validator;
mod directive_validator;
mod embedded;
mod id;
mod map;
mod relation;
mod unique_and_index;
mod updated_at;

use crate::dml;
use directive_list_validator::DirectiveListValidator;
use directive_validator::DirectiveValidator;

/// The argument type for directive validators.
type Args<'a> = crate::common::arguments::Arguments<'a>;

pub fn all_directives() -> AllDirectives {
    AllDirectives {
        field: new_builtin_field_directives(),
        model: new_builtin_model_directives(),
        enm: new_builtin_enum_directives(),
        enm_value: new_builtin_enum_value_directives(),
    }
}

/// convenience struct that contains all available directive validators
pub struct AllDirectives {
    pub field: DirectiveListValidator<dml::Field>,
    pub model: DirectiveListValidator<dml::Model>,
    pub enm: DirectiveListValidator<dml::Enum>,
    pub enm_value: DirectiveListValidator<dml::EnumValue>,
}

fn new_builtin_field_directives() -> DirectiveListValidator<dml::Field> {
    let mut validator = DirectiveListValidator::<dml::Field>::new();

    validator.add(Box::new(map::MapDirectiveValidatorForField {}));
    validator.add(Box::new(id::IdDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::FieldLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(default::DefaultDirectiveValidator {}));
    validator.add(Box::new(relation::RelationDirectiveValidator {}));
    validator.add(Box::new(updated_at::UpdatedAtDirectiveValidator {}));

    validator
}

fn new_builtin_model_directives() -> DirectiveListValidator<dml::Model> {
    let mut validator = DirectiveListValidator::<dml::Model>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));
    validator.add(Box::new(embedded::EmbeddedDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelIndexDirectiveValidator {}));
    validator.add(Box::new(id::ModelLevelIdDirectiveValidator {}));

    validator
}

fn new_builtin_enum_directives() -> DirectiveListValidator<dml::Enum> {
    let mut validator = DirectiveListValidator::<dml::Enum>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));

    validator
}

fn new_builtin_enum_value_directives() -> DirectiveListValidator<dml::EnumValue> {
    let mut validator = DirectiveListValidator::<dml::EnumValue>::new();

    validator.add(Box::new(map::MapDirectiveValidator {}));

    validator
}
