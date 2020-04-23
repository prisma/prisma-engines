use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml, IndexDefinition, IndexType};

/// Prismas builtin `@unique` directive.
pub struct FieldLevelUniqueDirectiveValidator {}

impl DirectiveValidator<dml::Field> for FieldLevelUniqueDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"unique"
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if obj.is_id {
            return self.new_directive_validation_error(
                "Fields that are marked as id should not have an additional @unique.",
                args.span(),
            );
        }

        if let dml::FieldType::Relation(_) = obj.field_type {
            return self.new_directive_validation_error(
                &format!(
                    "The field `{}` is a relation field and cannot be marked with `{}`. Only scalar fields can be made unique.",
                    &obj.name,
                    self.directive_name()
                ),
                args.span(),
            );
        }

        obj.is_unique = true;

        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if field.is_unique {
            return Ok(vec![ast::Directive::new(self.directive_name(), vec![])]);
        }

        Ok(vec![])
    }
}

/// Prismas builtin `@@unique` directive.
pub struct ModelLevelUniqueDirectiveValidator {}

impl IndexDirectiveBase<dml::Model> for ModelLevelUniqueDirectiveValidator {}
impl DirectiveValidator<dml::Model> for ModelLevelUniqueDirectiveValidator {
    fn directive_name(&self) -> &str {
        "unique"
    }

    fn is_duplicate_definition_allowed(&self) -> bool {
        true
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Unique)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(
        &self,
        model: &dml::Model,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        self.serialize_index_definitions(&model, IndexType::Unique)
    }
}

/// Prismas builtin `@@index` directive.
pub struct ModelLevelIndexDirectiveValidator {}

impl IndexDirectiveBase<dml::Model> for ModelLevelIndexDirectiveValidator {}
impl DirectiveValidator<dml::Model> for ModelLevelIndexDirectiveValidator {
    fn directive_name(&self) -> &str {
        "index"
    }

    fn is_duplicate_definition_allowed(&self) -> bool {
        true
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        let index_def = self.validate_index(args, obj, IndexType::Normal)?;
        obj.indices.push(index_def);

        Ok(())
    }

    fn serialize(
        &self,
        model: &dml::Model,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        self.serialize_index_definitions(&model, IndexType::Normal)
    }
}

/// common logic for `@@unique` and `@@index`
trait IndexDirectiveBase<T>: DirectiveValidator<T> {
    fn validate_index(
        &self,
        args: &mut Args,
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
            .filter(|field| match obj.find_field(&field) {
                Some(field) => field.field_type.is_relation(),
                None => false,
            })
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
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The {}index definition refers to the relation fields {}. Index definitions must reference only scalar fields.",
                    if index_type == IndexType::Unique { "unique " } else { "" },
                    referenced_relation_fields.join(", ")
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
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        let directives: Vec<ast::Directive> = model
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

                ast::Directive::new(self.directive_name(), args)
            })
            .collect();

        Ok(directives)
    }
}
