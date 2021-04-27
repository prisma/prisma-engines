use super::{super::helpers::*, AttributeValidator};
use crate::diagnostics::DatamodelError;
use crate::{ast, dml, PrimaryKeyDefinition};

/// Prismas builtin `@primary` attribute.
pub struct IdAttributeValidator {}

impl AttributeValidator<dml::Field> for IdAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"id"
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::ScalarField(sf) = obj {
            sf.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                fields: vec![sf.name.clone()],
            });
            Ok(())
        } else {
            self.new_attribute_validation_error(
                &format!(
                    "The field `{}` is a relation field and cannot be marked with `@{}`. Only scalar fields can be declared as id.",
                    &obj.name(),
                    self.attribute_name()
                ),
                args.span(),
            )
        }
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let dml::Field::ScalarField(sf) = field {
            if sf.primary_key.is_some() {
                return vec![ast::Attribute::new(self.attribute_name(), Vec::new())];
            }
        }

        vec![]
    }
}

pub struct ModelLevelIdAttributeValidator {}

impl AttributeValidator<dml::Model> for ModelLevelIdAttributeValidator {
    fn attribute_name(&self) -> &str {
        "id"
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        if obj.fields.iter().any(|f| f.is_id()) {
            return Err(DatamodelError::new_model_validation_error(
                "Each model must have at most one id criteria. You can't have `@id` and `@@id` at the same time.",
                &obj.name,
                args.span(),
            ));
        }

        let fields = args
            .default_arg("fields")?
            .as_array()
            .iter()
            .map(|f| f.as_constant_literal())
            .collect::<Result<Vec<_>, _>>()?;

        obj.primary_key = Some(PrimaryKeyDefinition {
            name: None,
            fields: fields.clone(),
        });

        let undefined_fields: Vec<String> = fields
            .iter()
            .filter_map(|field| {
                if obj.find_field(&field).is_none() {
                    Some(field.to_string())
                } else {
                    None
                }
            })
            .collect();

        let referenced_relation_fields: Vec<String> = fields
            .iter()
            .filter(|field| match obj.find_field(&field) {
                Some(field) => field.is_relation(),
                None => false,
            })
            .map(|f| f.to_owned())
            .collect();

        if !undefined_fields.is_empty() {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The multi field id declaration refers to the unknown fields {}.",
                    undefined_fields.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        if !referenced_relation_fields.is_empty() {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The id definition refers to the relation fields {}. Id definitions must reference only scalar fields.",
                    referenced_relation_fields.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        // the unwrap is safe because we error on undefined fields before
        let fields_that_are_not_required: Vec<_> = fields
            .iter()
            .filter(|field| !obj.find_field(&field).unwrap().arity().is_required())
            .map(|field| field.to_string())
            .collect();

        if !fields_that_are_not_required.is_empty() {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The id definition refers to the optional fields {}. Id definitions must reference only required fields.",
                    fields_that_are_not_required.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let Some(pk) = &model.primary_key {
            if model.has_compound_id() {
                let args = vec![ast::Argument::new_array(
                    "",
                    pk.fields
                        .iter()
                        .map(|f| ast::Expression::ConstantValue(f.to_string(), ast::Span::empty()))
                        .collect(),
                )];

                return vec![ast::Attribute::new(self.attribute_name(), args)];
            }
        }

        vec![]
    }
}
