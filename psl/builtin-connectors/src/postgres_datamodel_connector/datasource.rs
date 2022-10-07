use super::PostgresExtensions;
use crate::postgres_datamodel_connector::PostgresExtension;
use psl_core::{
    datamodel_connector::EXTENSIONS_KEY,
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{ast, coerce, coerce_array},
};
use std::collections::{HashMap, HashSet};

pub(super) fn parse_extensions(
    args: &mut HashMap<&str, (ast::Span, &ast::Expression)>,
    diagnostics: &mut Diagnostics,
) -> Option<PostgresExtensions> {
    args.remove(EXTENSIONS_KEY).and_then(|(span, expr)| {
        let mut extensions = Vec::new();

        for (name, args, span) in coerce_array(expr, &coerce::function_or_constant_with_span, diagnostics)? {
            let mut args = filter_args(args, diagnostics);

            let db_name = fetch_string_arg(&mut args, "map", diagnostics);
            let schema = fetch_string_arg(&mut args, "schema", diagnostics);
            let version = fetch_string_arg(&mut args, "version", diagnostics);

            for (name, (span, _)) in args.into_iter() {
                diagnostics.push_error(DatamodelError::new_argument_not_known_error(name, span));
            }

            let extension = PostgresExtension {
                name: name.to_string(),
                span,
                db_name,
                schema,
                version,
            };

            extensions.push(extension)
        }

        extensions.sort_by(|a, b| a.name.cmp(&b.name));

        Some(PostgresExtensions { extensions, span })
    })
}

fn filter_args<'a>(
    args: &'a [ast::Argument],
    diagnostics: &mut Diagnostics,
) -> HashMap<&'a str, (ast::Span, Option<&'a str>)> {
    let mut dups = HashSet::new();

    args.iter()
        .filter_map(|arg| match arg.name.as_ref() {
            Some(name) if dups.contains(name.name.as_str()) => {
                diagnostics.push_error(DatamodelError::new_validation_error(
                    &format!("The argument `{}` can only be defined once", name.name),
                    arg.span,
                ));

                None
            }
            Some(name) => {
                dups.insert(name.name.as_str());
                Some((name.name.as_str(), (arg.span, coerce::string(&arg.value, diagnostics))))
            }
            None => {
                diagnostics.push_error(DatamodelError::new_validation_error(
                    "The argument must have a name",
                    arg.span,
                ));

                None
            }
        })
        .collect()
}

fn fetch_string_arg(
    args: &mut HashMap<&str, (ast::Span, Option<&str>)>,
    name: &str,
    diagnostics: &mut Diagnostics,
) -> Option<String> {
    match args.remove(name) {
        Some((_, Some(val))) => Some(val.to_string()),
        Some((span, None)) => {
            diagnostics.push_error(DatamodelError::new_validation_error(
                &format!("The `{name}` argument must be a string literal"),
                span,
            ));

            None
        }
        None => None,
    }
}
