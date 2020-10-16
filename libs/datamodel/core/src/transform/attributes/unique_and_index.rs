use super::{super::helpers::*, AttributeValidator};
use crate::diagnostics::DatamodelError;
use crate::{ast, dml, IndexDefinition, IndexType};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Prismas builtin `@unique` attribute.
pub struct FieldLevelUniqueAttributeValidator {}

impl AttributeValidator<dml::Field> for FieldLevelUniqueAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"unique"
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
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
            if sf.is_id {
                return self.new_attribute_validation_error(
                    "Fields that are marked as id should not have an additional @unique.",
                    args.span(),
                );
            } else {
                sf.is_unique = true;
            }
        }
        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Attribute>, DatamodelError> {
        if let dml::Field::ScalarField(sf) = field {
            if sf.is_unique {
                return Ok(vec![ast::Attribute::new(self.attribute_name(), vec![])]);
            }
        }
        Ok(vec![])
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

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Unique)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(
        &self,
        model: &dml::Model,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Attribute>, DatamodelError> {
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

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Normal)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(
        &self,
        model: &dml::Model,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Attribute>, DatamodelError> {
        self.serialize_index_definitions(&model, IndexType::Normal)
    }
}

/// common logic for `@@unique` and `@@index`
trait IndexAttributeBase<T>: AttributeValidator<T> {
    fn validate_index(
        &self,
        args: &mut Arguments,
        obj: &mut dml::Model,
        index_type: IndexType,
    ) -> Result<IndexDefinition, DatamodelError> {
        let mut index_def = IndexDefinition {
            name: None,
            fields: vec![],
            tpe: index_type,
        };
        let name = match args.optional_arg("name") {
            Some(name_arg) => Some(name_arg.as_str()?),
            None => None,
        };
        index_def.name = name;

        let fields = args
            .default_arg("fields")?
            .as_array()
            .iter()
            .map(|f| f.as_constant_literal().unwrap())
            .collect();
        index_def.fields = fields;

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

    fn serialize_index_definitions(
        &self,
        model: &dml::Model,
        index_type: IndexType,
    ) -> Result<Vec<ast::Attribute>, DatamodelError> {
        let attributes: Vec<ast::Attribute> = model
            .indices
            .iter()
            .filter(|index| index.tpe == index_type)
            .map(|index_def| {
                let mut args = Vec::new();

                args.push(ast::Argument::new_array(
                    "",
                    index_def
                        .fields
                        .iter()
                        .map(|f| ast::Expression::ConstantValue(f.to_string(), ast::Span::empty()))
                        .collect(),
                ));
                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", &name));
                }

                ast::Attribute::new(self.attribute_name(), args)
            })
            .collect();

        Ok(attributes)
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
