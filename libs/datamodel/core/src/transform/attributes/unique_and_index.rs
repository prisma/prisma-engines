use super::AttributeValidator;
use crate::{ast, dml, IndexType};

/// Prismas builtin `@unique` attribute.
pub struct FieldLevelUniqueAttributeValidator {}

impl AttributeValidator<dml::Field> for FieldLevelUniqueAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "unique"
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let dml::Field::ScalarField(sf) = field {
            if sf.is_unique {
                return vec![ast::Attribute::new(self.attribute_name(), vec![])];
            }
        }

        vec![]
    }
}

/// Prismas builtin `@@unique` attribute.
pub struct ModelLevelUniqueAttributeValidator {}

impl IndexAttributeBase<dml::Model> for ModelLevelUniqueAttributeValidator {}
impl AttributeValidator<dml::Model> for ModelLevelUniqueAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "unique"
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        self.serialize_index_definitions(model, IndexType::Unique)
    }
}

/// Prismas builtin `@@index` attribute.
pub struct ModelLevelIndexAttributeValidator {}

impl IndexAttributeBase<dml::Model> for ModelLevelIndexAttributeValidator {}
impl AttributeValidator<dml::Model> for ModelLevelIndexAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "index"
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        self.serialize_index_definitions(model, IndexType::Normal)
    }
}

/// common logic for `@@unique` and `@@index`
trait IndexAttributeBase<T>: AttributeValidator<T> {
    fn serialize_index_definitions(&self, model: &dml::Model, index_type: IndexType) -> Vec<ast::Attribute> {
        let attributes: Vec<ast::Attribute> = model
            .indices
            .iter()
            .filter(|index| index.tpe == index_type)
            .map(|index_def| {
                let mut args = vec![ast::Argument::new_array(
                    "",
                    index_def
                        .fields
                        .iter()
                        .map(|f| ast::Expression::ConstantValue(f.to_string(), ast::Span::empty()))
                        .collect(),
                )];

                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", name));
                }

                ast::Attribute::new(self.attribute_name(), args)
            })
            .collect();

        attributes
    }
}
