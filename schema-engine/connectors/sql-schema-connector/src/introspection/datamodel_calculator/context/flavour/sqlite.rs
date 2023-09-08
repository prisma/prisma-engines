use std::borrow::Cow;

use sql_schema_describer as sql;
use sqlparser::{ast::Statement, parser::Parser};

pub(crate) struct SqliteIntrospectionFlavour;

impl super::IntrospectionFlavour for SqliteIntrospectionFlavour {
    fn keep_previous_scalar_field_arity(&self, next: sql::ColumnWalker<'_>) -> bool {
        next.is_in_view() && next.column_type().arity.is_nullable()
    }

    fn format_view_definition(&self, definition: &str) -> String {
        let dialect = sqlparser::dialect::SQLiteDialect {};

        let stmt = Parser::new(&dialect)
            .try_with_sql(definition)
            .and_then(|mut p| p.parse_statement());

        let definition = match stmt {
            // SQLite stores the definition as `CREATE VIEW`,
            // but we only want the query part in the definition
            // file as we do with the other databases.
            Ok(Statement::CreateView { query, .. }) => Cow::Owned(format!("{query};")),
            // If we get anything else than `CREATE VIEW`,
            // we just print it to a file and hope to get an issue
            // filed if it's wrong.
            _ => Cow::Borrowed(definition),
        };

        let opts = sqlformat::FormatOptions {
            uppercase: true,
            ..Default::default()
        };

        sqlformat::format(&definition, &Default::default(), opts)
    }
}
