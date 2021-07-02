use super::AttributeValidator;
use crate::{ast, dml};

/// Prismas builtin `@primary` attribute.
pub struct IdAttributeValidator {}

impl AttributeValidator<dml::Field> for IdAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "id"
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let dml::Field::ScalarField(sf) = field {
            if sf.is_id {
                return vec![ast::Attribute::new(self.attribute_name(), Vec::new())];
            }
        }

        vec![]
    }
}

pub struct ModelLevelIdAttributeValidator {}

impl AttributeValidator<dml::Model> for ModelLevelIdAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "id"
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if !model.id_fields.is_empty() {
            let args = vec![ast::Argument::new_array(
                "",
                model
                    .id_fields
                    .iter()
                    .map(|f| ast::Expression::ConstantValue(f.to_string(), ast::Span::empty()))
                    .collect(),
            )];

            return vec![ast::Attribute::new(self.attribute_name(), args)];
        }

        vec![]
    }
}
