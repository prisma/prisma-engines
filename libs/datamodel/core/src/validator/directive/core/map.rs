use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml, DatabaseName};

/// Prismas builtin `@map` directive.
pub struct MapDirectiveValidator {}

impl<T: dml::WithDatabaseName> DirectiveValidator<T> for MapDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"map"
    }
    fn validate_and_apply(&self, args: &mut Args, obj: &mut T) -> Result<(), DatamodelError> {
        match args.default_arg("name")?.as_array() {
            Ok(value) => match value.len() {
                0 => panic!("needs to be at least 1"),
                1 => obj.set_database_names(Some(DatabaseName::Single(value[0].as_str()?))),
                _ => obj.set_database_names(Some(DatabaseName::Compound(
                    value
                        .into_iter()
                        .map(|v| v.as_str())
                        .collect::<Result<Vec<String>, _>>()?,
                ))),
            },
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
        if let Some(db_name) = obj.database_names() {
            //todo can be single or compound
            match db_name {
                DatabaseName::Single(name) => {
                    return Ok(vec![ast::Directive::new(
                        DirectiveValidator::<T>::directive_name(self),
                        vec![ast::Argument::new_string("", name)],
                    )])
                }
                DatabaseName::Compound(names) => {
                    let mut related_fields: Vec<ast::Expression> = Vec::new();
                    for related_field in names {
                        related_fields.push(ast::Expression::ConstantValue(
                            related_field.clone(),
                            ast::Span::empty(),
                        ));
                    }

                    return Ok(vec![ast::Directive::new(
                        DirectiveValidator::<T>::directive_name(self),
                        vec![ast::Argument::new_array("", related_fields)],
                    )]);
                }
            }
        }

        Ok(vec![])
    }
}
