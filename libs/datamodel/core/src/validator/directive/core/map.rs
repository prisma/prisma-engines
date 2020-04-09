use crate::ast::Span;
use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml, WithDatabaseName};

/// Prismas builtin `@map` directive.
pub struct MapDirectiveValidator {}

const DIRECTIVE_NAME: &'static str = "map";

impl<T: dml::WithDatabaseName> DirectiveValidator<T> for MapDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut T) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &T, _datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, DatamodelError> {
        internal_serialize(obj)
    }
}

fn internal_validate_and_apply(args: &mut Args, obj: &mut dyn WithDatabaseName) -> Result<(), DatamodelError> {
    let db_name = args.default_arg("name")?.as_str().map_err(|err| {
        DatamodelError::new_directive_validation_error(&format!("{}", err), DIRECTIVE_NAME, err.span())
    })?;
    obj.set_database_name(Some(db_name));
    Ok(())
}

fn internal_serialize(obj: &dyn WithDatabaseName) -> Result<Vec<ast::Directive>, DatamodelError> {
    match obj.single_database_name() {
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
