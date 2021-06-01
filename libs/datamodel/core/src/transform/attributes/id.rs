use super::{super::helpers::*, AttributeValidator};
use crate::diagnostics::DatamodelError;
use crate::transform::attributes::field_array;
use crate::{ast, dml, PrimaryKeyDefinition};
use once_cell::sync::Lazy;
use regex::Regex;

/// Prismas builtin `@primary` attribute.
pub struct IdAttributeValidator {}

impl AttributeValidator<dml::Field> for IdAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"id"
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::ScalarField(sf) = obj {
            let name_in_db = args
                .optional_default_arg("map")?
                .map(|v| v.as_str().unwrap().to_string());

            sf.primary_key = Some(PrimaryKeyDefinition {
                // if this is none, the default name needs to be set one level higher since we do not have the model name here -.-
                name_in_db,
                name_in_db_matches_default: false,
                name_in_client: None,
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
            if let Some(pk) = &sf.primary_key {
                let arguments = if !pk.name_in_db_matches_default && pk.name_in_db.is_some() {
                    vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                        pk.name_in_db.clone().unwrap(),
                        ast::Span::empty(),
                    ))]
                } else {
                    vec![]
                };
                return vec![ast::Attribute::new(self.attribute_name(), arguments)];
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

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Model) -> Result<(), DatamodelError> {
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

        let (name_in_client, name_in_db) = match (
            args.optional_arg("name").map(|v| v.as_str().unwrap().to_string()),
            args.optional_arg("map").map(|v| v.as_str().unwrap().to_string()),
        ) {
            (Some(client_name), Some(db_name)) => (Some(client_name), Some(db_name)),
            (Some(client_name), None) => (Some(client_name), None),
            (None, Some(db_name)) => (None, Some(db_name)),
            (None, None) => (None, None),
        };

        static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

        if let Some(name) = &name_in_client {
            if RE.is_match(&name) {
                return Err(DatamodelError::new_model_validation_error(
                    "The `name` property within the `@@id` attribute only allows for the following characters: `_a-zA-Z0-9`.",
                    &obj.name,
                    args.span(),
                ));
            }
        }

        obj.primary_key = Some(PrimaryKeyDefinition {
            name_in_client,
            name_in_db_matches_default: false,
            name_in_db,
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
            if model.singular_id_fields().next().is_none() {
                let mut args = vec![ast::Argument::new_array("", field_array(&pk.fields))];

                if let Some(name) = &pk.name_in_client {
                    args.push(ast::Argument::new(
                        "name",
                        ast::Expression::StringValue(name.to_string(), ast::Span::empty()),
                    ))
                }

                if let Some(name) = &pk.name_in_db {
                    if !pk.name_in_db_matches_default {
                        args.push(ast::Argument::new(
                            "map",
                            ast::Expression::StringValue(name.to_string(), ast::Span::empty()),
                        ))
                    }
                }

                return vec![ast::Attribute::new(self.attribute_name(), args)];
            }
        }

        vec![]
    }
}
