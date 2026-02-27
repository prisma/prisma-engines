use std::borrow::Cow;

use sqlparser::{
    ast::{CreateView, Statement},
    dialect::MsSqlDialect,
    parser::Parser,
};

pub(crate) struct SqlServerIntrospectionFlavour;

impl super::IntrospectionFlavour for SqlServerIntrospectionFlavour {
    fn format_view_definition(&self, definition: &str) -> String {
        let dialect = MsSqlDialect {};

        let stmt = Parser::new(&dialect)
            .try_with_sql(definition)
            .and_then(|mut p| p.parse_statement());

        let definition = match stmt {
            // SQL Server stores the definition as `CREATE VIEW`,
            // but we only want the query part in the definition
            // file as we do with the other databases.
            Ok(Statement::CreateView(CreateView { query, .. })) => Cow::Owned(format!("{query};")),
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
