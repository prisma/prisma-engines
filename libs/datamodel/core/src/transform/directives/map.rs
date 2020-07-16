use super::{super::helpers::*, DirectiveValidator};
use crate::ast::{Directive, Span};
use crate::error::DatamodelError;
use crate::{ast, dml, Datamodel, WithDatabaseName};

/// Prismas builtin `@map` directive.
pub struct MapDirectiveValidator {}

const DIRECTIVE_NAME: &str = "map";

impl DirectiveValidator<dml::Model> for MapDirectiveValidator {
    fn directive_name(&self) -> &str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Result<Vec<Directive>, DatamodelError> {
        internal_serialize(obj)
    }
}

pub struct MapDirectiveValidatorForField {}
impl DirectiveValidator<dml::Field> for MapDirectiveValidatorForField {
    fn directive_name(&self) -> &str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if obj.is_relation() {
            return self.new_directive_validation_error(
                &format!(
                    "The directive `@{}` can not be used on relation fields.",
                    DIRECTIVE_NAME
                ),
                args.span(),
            );
        }
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Field, _datamodel: &Datamodel) -> Result<Vec<Directive>, DatamodelError> {
        internal_serialize(obj)
    }
}

impl DirectiveValidator<dml::Enum> for MapDirectiveValidator {
    fn directive_name(&self) -> &str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Enum) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Enum, _datamodel: &Datamodel) -> Result<Vec<Directive>, DatamodelError> {
        internal_serialize(obj)
    }
}

impl DirectiveValidator<dml::EnumValue> for MapDirectiveValidator {
    fn directive_name(&self) -> &str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::EnumValue) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::EnumValue, _datamodel: &Datamodel) -> Result<Vec<Directive>, DatamodelError> {
        internal_serialize(obj)
    }
}

fn internal_validate_and_apply(args: &mut Arguments, obj: &mut dyn WithDatabaseName) -> Result<(), DatamodelError> {
    let db_name = args.default_arg("name")?.as_str().map_err(|err| {
        DatamodelError::new_directive_validation_error(&format!("{}", err), DIRECTIVE_NAME, err.span())
    })?;
    obj.set_database_name(Some(db_name));
    Ok(())
}

fn internal_serialize(obj: &dyn WithDatabaseName) -> Result<Vec<ast::Directive>, DatamodelError> {
    match obj.database_name() {
        Some(db_name) => Ok(vec![ast::Directive::new(
            DIRECTIVE_NAME,
            vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                String::from(db_name),
                Span::empty(),
            ))],
        )]),
        None => Ok(vec![]),
    }
}
