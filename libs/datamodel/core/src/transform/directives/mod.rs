mod default;
mod directive_list_validator;
mod directive_validator;
mod id;
mod map;
mod relation;
mod unique_and_index;
mod updated_at;

use crate::dml;
use directive_list_validator::DirectiveListValidator;
use directive_validator::DirectiveValidator;

/// This is the facade for all directive validations. It is used within the `ValidationPipeline`.
pub struct AllDirectives {
    pub field: DirectiveListValidator<dml::Field>,
    pub model: DirectiveListValidator<dml::Model>,
    pub enm: DirectiveListValidator<dml::Enum>,
    pub enm_value: DirectiveListValidator<dml::EnumValue>,
}

impl AllDirectives {
    pub fn new() -> AllDirectives {
        AllDirectives {
            field: new_builtin_field_directives(),
            model: new_builtin_model_directives(),
            enm: new_builtin_enum_directives(),
            enm_value: new_builtin_enum_value_directives(),
        }
    }
}

fn new_builtin_field_directives() -> DirectiveListValidator<dml::Field> {
    let mut validator = DirectiveListValidator::<dml::Field>::new();

    // this order of field attributes is used in the formatter as well
    validator.add(Box::new(id::IdDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::FieldLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(default::DefaultDirectiveValidator {}));
    validator.add(Box::new(updated_at::UpdatedAtDirectiveValidator {}));
    validator.add(Box::new(map::MapDirectiveValidatorForField {}));
    validator.add(Box::new(relation::RelationDirectiveValidator {}));

    validator
}

fn new_builtin_model_directives() -> DirectiveListValidator<dml::Model> {
    let mut validator = DirectiveListValidator::<dml::Model>::new();

    // this order of block attributes is used in the formatter as well
    validator.add(Box::new(id::ModelLevelIdDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelUniqueDirectiveValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelIndexDirectiveValidator {}));
    validator.add(Box::new(map::MapDirectiveValidator {}));

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
