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
        let db_names: Vec<ast::Expression> = obj
            .database_names()
            .into_iter()
            .map(|name| ast::Expression::StringValue(String::from(name), Span::empty()))
            .collect();

        match db_names.len() {
            0 => Ok(vec![]),
            1 => {
                let db_name = db_names.into_iter().next().unwrap();
                Ok(vec![ast::Directive::new(
                    DIRECTIVE_NAME,
                    vec![ast::Argument::new("", db_name)],
                )])
            }
            _ => {
                let directive = ast::Directive::new(DIRECTIVE_NAME, vec![ast::Argument::new_array("", db_names)]);
                Ok(vec![directive])
            }
        }
    }
}

fn internal_validate_and_apply(args: &mut Args, obj: &mut dyn WithDatabaseName) -> Result<(), DatamodelError> {
    let db_name = args.default_arg("name")?.as_str().map_err(|err| {
        DatamodelError::new_directive_validation_error(&format!("{}", err), DIRECTIVE_NAME, err.span())
    })?;
    obj.set_database_name(Some(db_name));
    Ok(())
}
