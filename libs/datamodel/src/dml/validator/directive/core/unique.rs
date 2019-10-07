use crate::dml::validator::directive::{Args, DirectiveValidator, Error};
use crate::{ast, dml, IndexDefinition, IndexType};

/// Prismas builtin `@unique` directive.
pub struct UniqueDirectiveValidator {}

impl DirectiveValidator<dml::Field> for UniqueDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"unique"
    }

    fn validate_and_apply(&self, _args: &mut Args, obj: &mut dml::Field) -> Result<(), Error> {
        obj.is_unique = true;

        Ok(())
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Result<Option<ast::Directive>, Error> {
        if field.is_unique {
            return Ok(Some(ast::Directive::new(self.directive_name(), vec![])));
        }

        Ok(None)
    }
}

/// Prismas builtin `@@unique` directive.
pub struct ModelLevelUniqueValidator {}

impl DirectiveValidator<dml::Model> for ModelLevelUniqueValidator {
    fn directive_name(&self) -> &str {
        "unique"
    }

    fn is_duplicate_definition_allowed(&self) -> bool {
        true
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Model) -> Result<(), Error> {
        let mut index_def = IndexDefinition {
            name: None,
            fields: vec![],
            tpe: IndexType::Unique,
        };
        //        let name = args.optional_arg("name").map(|name_arg| name_arg?.as_str()?);
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
            Err(err) => return self.parser_error(&err),
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
                    "The unique index definition refers to the unknown fields {}.",
                    undefined_fields.join(", ")
                ),
                &obj.name,
                args.span(),
            ));
        }

        obj.indexes.push(index_def);

        Ok(())
    }

    fn serialize(&self, model: &dml::Model, _datamodel: &dml::Datamodel) -> Result<Option<ast::Directive>, Error> {
        let directives: Vec<ast::Directive> = model
            .indexes
            .iter()
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
        Ok(directives.first().cloned())
    }
}
