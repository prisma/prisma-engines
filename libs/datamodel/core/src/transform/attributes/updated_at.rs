use super::AttributeValidator;
use crate::{ast, dml};

/// Prismas builtin `@updatedAt` attribute.
pub struct UpdatedAtAttributeValidator {}

impl AttributeValidator<dml::Field> for UpdatedAtAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "updatedAt"
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if field.is_updated_at() {
            vec![ast::Attribute::new(self.attribute_name(), Vec::new())]
        } else {
            vec![]
        }
    }
}
