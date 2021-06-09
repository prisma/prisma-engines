#![deny(missing_docs)]

use super::{super::helpers::Arguments, AttributeValidator};
use crate::transform::attributes::field_array;
use crate::{ast, diagnostics::DatamodelError, dml, transform::helpers::ValueValidator, IndexDefinition, IndexType};
use once_cell::sync::Lazy;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Prismas builtin `@unique` attribute.
pub struct FieldLevelUniqueAttributeValidator {}

impl AttributeValidator<dml::Field> for FieldLevelUniqueAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"unique"
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::RelationField(rf) = obj {
            let suggestion = match rf.relation_info.fields.len().cmp(&1) {
                Ordering::Equal => format!(
                    " Did you mean to put it on `{}`?",
                    rf.relation_info.fields.first().unwrap()
                ),
                Ordering::Greater => format!(
                    " Did you mean to provide `@@unique([{}])`?",
                    rf.relation_info.fields.join(", ")
                ),
                // no suggestion possible
                Ordering::Less => String::new(),
            };

            return self.new_attribute_validation_error(
                &format!(
                    "The field `{field_name}` is a relation field and cannot be marked with `{attribute_name}`. Only scalar fields can be made unique.{suggestion}",
                    field_name = rf.name,
                    attribute_name  = self.attribute_name(),
                    suggestion = suggestion
                ),
                args.span(),
            );
        } else if let dml::Field::ScalarField(sf) = obj {
            if sf.primary_key.is_some() {
                return self.new_attribute_validation_error(
                    "Fields that are marked as id should not have an additional @unique.",
                    args.span(),
                );
            } else {
                let name_in_db = match args
                    .optional_default_arg("map")?
                    .map(|v| v.as_string_literal().map(|(str, span)| (str.to_owned(), span)))
                    .flatten()
                {
                    Some((x, span)) if x.is_empty() => {
                        return Err(DatamodelError::new_attribute_validation_error(
                            "The `map` argument cannot be an empty string.",
                            self.attribute_name(),
                            span,
                        ))
                    }
                    Some((map, _)) => map,
                    None => "".to_string(),
                };

                sf.is_unique = Some(IndexDefinition {
                    name_in_db,
                    name_in_db_matches_default: false,
                    name_in_client: None,
                    fields: vec![sf.name.clone()],
                    tpe: IndexType::Unique,
                });
            }
        }

        Ok(())
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let dml::Field::ScalarField(sf) = field {
            if let Some(unique) = &sf.is_unique {
                let arguments = if unique.name_in_db_matches_default {
                    vec![]
                } else {
                    vec![ast::Argument::new_string("", &unique.name_in_db)]
                };

                return vec![ast::Attribute::new(self.attribute_name(), arguments)];
            }
        }

        vec![]
    }
}

/// Prismas builtin `@@unique` attribute.
pub struct ModelLevelUniqueAttributeValidator {}

impl IndexAttributeBase<dml::Model> for ModelLevelUniqueAttributeValidator {}
impl AttributeValidator<dml::Model> for ModelLevelUniqueAttributeValidator {
    fn attribute_name(&self) -> &str {
        "unique"
    }

    fn is_duplicate_definition_allowed(&self) -> bool {
        true
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Unique)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        self.serialize_index_definitions(&model, IndexType::Unique)
    }
}

/// Prismas builtin `@@index` attribute.
pub struct ModelLevelIndexAttributeValidator {}

impl IndexAttributeBase<dml::Model> for ModelLevelIndexAttributeValidator {}
impl AttributeValidator<dml::Model> for ModelLevelIndexAttributeValidator {
    fn attribute_name(&self) -> &str {
        "index"
    }

    fn is_duplicate_definition_allowed(&self) -> bool {
        true
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Normal)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        self.serialize_index_definitions(&model, IndexType::Normal)
    }
}

/// common logic for `@@unique` and `@@index`
trait IndexAttributeBase<T>: AttributeValidator<T> {
    fn validate_index(
        &self,
        args: &mut Arguments<'_>,
        obj: &mut dml::Model,
        index_type: IndexType,
    ) -> Result<IndexDefinition, DatamodelError> {
        let fields = args
            .default_arg("fields")?
            .as_array()
            .iter()
            .map(|f| f.as_constant_literal())
            .collect::<Result<Vec<_>, _>>()?;

        let (name_in_client, name_in_db) = match (
            args.optional_arg("name")
                .as_ref()
                .and_then(ValueValidator::as_string_literal),
            args.optional_arg("map")
                .as_ref()
                .and_then(ValueValidator::as_string_literal),
        ) {
            (Some(("", span)), _) => {
                return Err(DatamodelError::new_attribute_validation_error(
                    "The `name` argument cannot be an empty string.",
                    self.attribute_name(),
                    span,
                ))
            }
            (_, Some(("", span))) => {
                return Err(DatamodelError::new_attribute_validation_error(
                    "The `map` argument cannot be an empty string.",
                    self.attribute_name(),
                    span,
                ))
            }
            (Some((name, _)), Some((map, _))) => (Some(name.to_owned()), map.to_owned()),
            //backwards compatibility, accept name arg on normal indexes and use it as map arg
            (Some((name, _)), None) if matches!(index_type, IndexType::Normal) => (None, name.to_owned()),
            (Some((name, _)), None) => (Some(name.to_owned()), "".to_string()),
            (None, Some((map, _))) => (None, map.to_owned()),
            (None, None) => (None, "".to_string()),
        };

        //only Alphanumeric characters and underscore are allowed due to this making its way into the client API
        //todo move this into the pest grammar at some point
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

        if let Some(name) = &name_in_client {
            if RE.is_match(&name) {
                return Err(DatamodelError::new_model_validation_error(
                    "The `name` property within the `@@unique` attribute only allows for the following characters: `_a-zA-Z0-9`.",
                    &obj.name,
                    args.span(),
                ));
            }
        }

        let index_def = IndexDefinition {
            name_in_client,
            name_in_db_matches_default: false,
            name_in_db,
            fields,
            tpe: index_type,
        };

        let duplicated_fields = find_duplicates(&index_def.fields);
        if !duplicated_fields.is_empty() {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The {}index definition refers to the fields {} multiple times.",
                    if index_type == IndexType::Unique { "unique " } else { "" },
                    duplicated_fields.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        let undefined_fields: Vec<String> = index_def
            .fields
            .iter()
            .filter_map(|field| {
                if obj.find_field(&field).is_none() {
                    Some(field.to_string())
                } else {
                    None
                }
            })
            .collect();

        let referenced_relation_fields: Vec<String> = index_def
            .fields
            .iter()
            .filter(|field| obj.find_relation_field(&field).is_some())
            .map(|f| f.to_owned())
            .collect();

        if !undefined_fields.is_empty() {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The {}index definition refers to the unknown fields {}.",
                    if index_type == IndexType::Unique { "unique " } else { "" },
                    undefined_fields.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        if !referenced_relation_fields.is_empty() {
            let mut suggested_fields = Vec::new();
            let mut had_successful_replacement = false;

            for f in &index_def.fields {
                if let Some(rf) = obj.find_relation_field(&f) {
                    for underlying_field in &rf.relation_info.fields {
                        suggested_fields.push(underlying_field.to_owned());
                        had_successful_replacement = true;
                    }
                }

                if let Some(sf) = obj.find_scalar_field(&f) {
                    suggested_fields.push(sf.name.clone());
                }
            }

            let suggestion = if had_successful_replacement {
                format!(
                    " Did you mean `@@{attribute_name}([{fields}])`?",
                    attribute_name = attribute_name(index_type),
                    fields = suggested_fields.join(", ")
                )
            } else {
                String::new()
            };

            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The {prefix}index definition refers to the relation fields {the_fields}. Index definitions must reference only scalar fields.{suggestion}",
                    prefix = if index_type == IndexType::Unique { "unique " } else { "" },
                    the_fields = referenced_relation_fields.join(", "),
                    suggestion = suggestion
                ),
                &obj.name,
                args.span(),
            ));
        }

        Ok(index_def)
    }

    fn serialize_index_definitions(&self, model: &dml::Model, index_type: IndexType) -> Vec<ast::Attribute> {
        let attributes: Vec<ast::Attribute> = model
            .indices
            .iter()
            .filter(|index| index.tpe == index_type)
            // no field level equivalent
            .filter(|index| {
                if index.fields.len() == 1 && index.tpe == IndexType::Unique {
                    let covered_field = index.fields.first().unwrap();
                    !model.find_field(covered_field).unwrap().is_unique()
                } else {
                    true
                }
            })
            .map(|index_def| {
                let mut args = vec![ast::Argument::new_array("", field_array(&index_def.fields))];

                if let Some(name) = &index_def.name_in_client {
                    args.push(ast::Argument::new_string("name", name));
                }
                if !index_def.name_in_db_matches_default {
                    args.push(ast::Argument::new_string("map", &index_def.name_in_db));
                }

                ast::Attribute::new(self.attribute_name(), args)
            })
            .collect();

        attributes
    }
}

fn attribute_name(index_type: dml::IndexType) -> &'static str {
    if index_type == dml::IndexType::Unique {
        "unique"
    } else {
        "index"
    }
}

// returns the items that are contained multiple times in the provided vector
fn find_duplicates(items: &[String]) -> Vec<String> {
    let mut counts = HashMap::new();
    for item in items.iter() {
        let entry = counts.entry(item).or_insert(0);
        *entry += 1;
    }

    let mut result = Vec::new();
    for (key, count) in counts.into_iter() {
        if count > 1 {
            result.push(key.to_owned());
        }
    }

    result
}
