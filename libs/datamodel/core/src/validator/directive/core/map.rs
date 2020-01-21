use crate::ast::Span;
use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml};

/// Prismas builtin `@map` directive.
pub struct MapDirectiveValidator {}

const DIRECTIVE_NAME: &'static str = "map";

impl<T: dml::WithDatabaseName> DirectiveValidator<T> for MapDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        DIRECTIVE_NAME
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut T) -> Result<(), DatamodelError> {
        match args.default_arg("name")?.as_array() {
            Ok(value) => {
                let db_names = value
                    .into_iter()
                    .map(|v| v.as_str())
                    .collect::<Result<Vec<String>, _>>()?;

                if db_names.len() == 0 {
                    return Err(DatamodelError::new_directive_validation_error(
                        "Expected one argument. No argument was provided.",
                        DIRECTIVE_NAME,
                        args.span(),
                    ));
                } else {
                    obj.set_database_names(db_names).map_err(|err_msg| {
                        DatamodelError::new_directive_validation_error(&err_msg, DIRECTIVE_NAME, args.span())
                    })?
                }
            }
            // self.parser_error would be better here, but we cannot call it due to rust limitations.
            Err(err) => {
                return Err(DatamodelError::new_directive_validation_error(
                    &format!("{}", err),
                    "map",
                    err.span(),
                ))
            }
        };

        Ok(())
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
