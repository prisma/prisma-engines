use crate::dml::validator::directive::{Args, DirectiveValidator, Error};
use crate::{ast, dml, IndexDefinition, IndexType};

/// Prismas builtin `@unique` directive.
pub struct FieldLevelUniqueDirectiveValidator {}

impl DirectiveValidator<dml::Field> for FieldLevelUniqueDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"unique"
    }

    fn validate_and_apply(&self, _args: &mut Args, obj: &mut dml::Field) -> Result<(), Error> {
        obj.is_unique = true;

        Ok(())
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, Error> {
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

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), Error> {
        let index_def = self.validate_index(args, obj, IndexType::Unique)?;
        obj.indexes.push(index_def);

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, Error> {
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

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), Error> {
        let index_def = self.validate_index(args, obj, IndexType::Normal)?;
        obj.indexes.push(index_def);

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, Error> {
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
    ) -> Result<IndexDefinition, Error> {
        let mut index_def = IndexDefinition {
            name: None,
            fields: vec![],
            tpe: index_type,
        };
        let name = match args.optional_arg("name") {
            Some(name_arg) => Some(name_arg?.as_str()?),
            None => None,
        };
        index_def.name = name;

        match args.default_arg("fields")?.as_array() {
            Ok(fields) => {
                let fields = fields.iter().map(|f| f.as_constant_literal().unwrap()).collect();
                index_def.fields = fields;
            }
            Err(err) => return Err(self.parser_error(&err)),
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

        if !undefined_fields.is_empty() {
            return Err(Error::new_model_validation_error(
                &format!(
                    "The {}index definition refers to the unknown fields {}.",
                    if index_type == IndexType::Unique { "unique " } else { "" },
                    undefined_fields.join(", ")
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
    ) -> Result<Vec<ast::Directive>, Error> {
        let directives: Vec<ast::Directive> = model
            .indexes
            .iter()
            .filter(|index| index.tpe == index_type)
            .map(|index_def| {
                let mut args = Vec::new();

                args.push(ast::Argument::new_array(
                    "",
                    index_def
                        .fields
                        .iter()
                        .map(|f| ast::Value::ConstantValue(f.to_string(), ast::Span::empty()))
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
