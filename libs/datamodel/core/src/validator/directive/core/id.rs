use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml};
use datamodel_connector::scalars::ScalarValue;

/// Prismas builtin `@primary` directive.
pub struct IdDirectiveValidator {}

impl DirectiveValidator<dml::Field> for IdDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"id"
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        let strategy = match (&obj.field_type, &obj.default_value) {
            (dml::FieldType::Base(dml::ScalarType::Int), _) => dml::IdStrategy::Auto,
            (dml::FieldType::Base(dml::ScalarType::String), Some(ScalarValue::Expression(_, _, _))) => {
                dml::IdStrategy::Auto
            }
            _ => dml::IdStrategy::None,
        };

        let id_info = dml::IdInfo {
            strategy,
            sequence: None,
        };

        if obj.arity != dml::FieldArity::Required {
            return self.error("Fields that are marked as id must be required.", args.span());
        }

        obj.id_info = Some(id_info);

        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if let Some(_) = &field.id_info {
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
            Err(err) => return Err(self.parser_error(&err)),
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
