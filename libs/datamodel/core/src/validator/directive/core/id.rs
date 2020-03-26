use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml};

/// Prismas builtin `@primary` directive.
pub struct IdDirectiveValidator {}

impl DirectiveValidator<dml::Field> for IdDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"id"
    }

    // TODO In which form is this still required or needs to change? Default values are handling the id strategy now.
    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if obj.arity != dml::FieldArity::Required {
            return self.new_directive_validation_error("Fields that are marked as id must be required.", args.span());
        }

        if let dml::FieldType::Relation(_) = obj.field_type {
            return self.new_directive_validation_error(
                &format!(
                    "The field `{}` is a relation field and cannot be marked with `@{}`. Only scalar fields can be declared as id.",
                    &obj.name,
                    self.directive_name()
                ),
                args.span(),
            );
        }

        obj.is_id = true;

        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if field.is_id {
            return Ok(vec![ast::Directive::new(self.directive_name(), Vec::new())]);
        }

        Ok(vec![])
    }
}

pub struct ModelLevelIdDirectiveValidator {}

impl DirectiveValidator<dml::Model> for ModelLevelIdDirectiveValidator {
    fn directive_name(&self) -> &str {
        "id"
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        match args.default_arg("fields")?.as_array() {
            Ok(fields) => {
                let fields = fields.iter().map(|f| f.as_constant_literal().unwrap()).collect();
                obj.id_fields = fields;
            }
            Err(err) => return Err(self.wrap_in_directive_validation_error(&err)),
        };

        let undefined_fields: Vec<String> = obj
            .id_fields
            .iter()
            .filter_map(|field| {
                if obj.find_field(&field).is_none() {
                    Some(field.to_string())
                } else {
                    None
                }
            })
            .collect();

        let referenced_relation_fields: Vec<String> = obj
            .id_fields
            .iter()
            .filter(|field| match obj.find_field(&field) {
                Some(field) => field.field_type.is_relation(),
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

        Ok(())
    }

    fn serialize(
        &self,
        model: &dml::Model,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if !model.id_fields.is_empty() {
            let mut args = Vec::new();

            args.push(ast::Argument::new_array(
                "",
                model
                    .id_fields
                    .iter()
                    .map(|f| ast::Expression::ConstantValue(f.to_string(), ast::Span::empty()))
                    .collect(),
            ));

            return Ok(vec![ast::Directive::new(self.directive_name(), args)]);
        }

        Ok(vec![])
    }
}
