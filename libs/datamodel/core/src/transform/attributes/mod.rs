mod attribute_list_validator;
mod attribute_validator;
mod default;
mod id;
mod map;
mod relation;
mod unique_and_index;
mod updated_at;

use crate::dml;
use attribute_list_validator::AttributeListValidator;
use attribute_validator::AttributeValidator;

/// This is the facade for all attribute validations. It is used within the `ValidationPipeline`.
pub struct AllAttributes {
    pub field: AttributeListValidator<dml::Field>,
    pub model: AttributeListValidator<dml::Model>,
    pub enm: AttributeListValidator<dml::Enum>,
    pub enm_value: AttributeListValidator<dml::EnumValue>,
}

impl AllAttributes {
    pub fn new() -> AllAttributes {
        AllAttributes {
            field: new_builtin_field_attributes(),
            model: new_builtin_model_attributes(),
            enm: new_builtin_enum_attributes(),
            enm_value: new_builtin_enum_value_attributes(),
        }
    }
}

fn new_builtin_field_attributes() -> AttributeListValidator<dml::Field> {
    let mut validator = AttributeListValidator::<dml::Field>::new();

    // this order of field attributes is used in the formatter as well
    validator.add(Box::new(id::IdAttributeValidator {}));
    validator.add(Box::new(unique_and_index::FieldLevelUniqueAttributeValidator {}));
    validator.add(Box::new(default::DefaultAttributeValidator {}));
    validator.add(Box::new(updated_at::UpdatedAtAttributeValidator {}));
    validator.add(Box::new(map::MapAttributeValidatorForField {}));
    validator.add(Box::new(relation::RelationAttributeValidator {}));

    validator
}

fn new_builtin_model_attributes() -> AttributeListValidator<dml::Model> {
    let mut validator = AttributeListValidator::<dml::Model>::new();

    // this order of block attributes is used in the formatter as well
    validator.add(Box::new(id::ModelLevelIdAttributeValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelUniqueAttributeValidator {}));
    validator.add(Box::new(unique_and_index::ModelLevelIndexAttributeValidator {}));
    validator.add(Box::new(map::MapAttributeValidator {}));

    validator
}

fn new_builtin_enum_attributes() -> AttributeListValidator<dml::Enum> {
    let mut validator = AttributeListValidator::<dml::Enum>::new();

    validator.add(Box::new(map::MapAttributeValidator {}));

    validator
}

fn new_builtin_enum_value_attributes() -> AttributeListValidator<dml::EnumValue> {
    let mut validator = AttributeListValidator::<dml::EnumValue>::new();

    validator.add(Box::new(map::MapAttributeValidator {}));

    validator
}
