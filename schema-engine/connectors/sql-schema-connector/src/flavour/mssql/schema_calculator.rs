use psl::{
    datamodel_connector::walker_ext_traits::{DefaultValueExt, IndexWalkerExt},
    parser_database::walkers::*,
};
use sql_schema_describer::{
    ForeignKeyAction,
    mssql::{IndexBits, MssqlSchemaExt},
};

use crate::sql_schema_calculator::SqlSchemaCalculatorFlavour;

#[derive(Debug, Default)]
pub struct MssqlSchemaCalculatorFlavour;

impl SqlSchemaCalculatorFlavour for MssqlSchemaCalculatorFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::MSSQL
    }

    fn default_constraint_name(&self, default_value: DefaultValueWalker<'_>) -> Option<String> {
        Some(default_value.constraint_name(self.datamodel_connector()).into_owned())
    }

    fn normalize_index_predicate(&self, predicate: String, is_raw: bool) -> String {
        let predicate = if is_raw {
            predicate
        } else {
            let predicate = replace_identifier_quotes(&predicate);

            predicate
                .replace(" = true", "=(1)")
                .replace(" = false", "=(0)")
                .replace(" != true", "!=(1)")
                .replace(" != false", "!=(0)")
        };

        if predicate.starts_with('(') && predicate.ends_with(')') {
            predicate
        } else {
            format!("({predicate})")
        }
    }

    fn m2m_foreign_key_action(&self, model_a: ModelWalker<'_>, model_b: ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }

    fn push_connector_data(&self, context: &mut crate::sql_schema_calculator::Context<'_>) {
        let mut data = MssqlSchemaExt::default();

        for model in context.datamodel.db.walk_models() {
            let table_id = context.model_id_to_table_id[&model.id];
            let table = context.schema.walk(table_id);
            if model
                .primary_key()
                .map(|pk| pk.clustered().is_none() || pk.clustered() == Some(true))
                .unwrap_or(false)
            {
                *data.index_bits.entry(table.primary_key().unwrap().id).or_default() |= IndexBits::Clustered;
            }

            for index in model.indexes() {
                let sql_index = table
                    .indexes()
                    .find(|idx| idx.name() == index.constraint_name(self.datamodel_connector()))
                    .unwrap();

                if index.clustered() == Some(true) {
                    *data.index_bits.entry(sql_index.id).or_default() |= IndexBits::Clustered;
                }
            }
        }

        context.schema.describer_schema.set_connector_data(Box::new(data));
    }
}

/// Replace `"` identifier quotes with MSSQL `[]` brackets using the SQL parser.
fn replace_identifier_quotes(sql: &str) -> String {
    use std::ops::ControlFlow;

    use sqlparser::{
        ast::{Expr, Ident, VisitMut, VisitorMut},
        dialect::GenericDialect,
        parser::Parser,
    };

    struct QuoteConverter;

    impl VisitorMut for QuoteConverter {
        type Break = ();

        fn post_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
            match expr {
                Expr::Identifier(ident) => {
                    convert_ident(ident);
                }
                Expr::CompoundIdentifier(idents) => {
                    idents.iter_mut().for_each(convert_ident);
                }
                _ => {}
            }
            ControlFlow::Continue(())
        }
    }

    fn convert_ident(ident: &mut Ident) {
        if ident.quote_style == Some('"') {
            ident.quote_style = Some('[');
        }
    }

    let dialect = GenericDialect {};
    match Parser::new(&dialect).try_with_sql(sql).and_then(|mut p| p.parse_expr()) {
        Ok(mut expr) => {
            let _ = expr.visit(&mut QuoteConverter);
            expr.to_string()
        }
        Err(_) => sql.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_identifier_quotes() {
        assert_eq!(replace_identifier_quotes(r#""col" = 1"#), "[col] = 1");
    }

    #[test]
    fn preserves_quotes_inside_string_literals() {
        assert_eq!(
            replace_identifier_quotes(r#""col" = '"whatever"'"#),
            r#"[col] = '"whatever"'"#
        );
    }

    #[test]
    fn handles_escaped_single_quotes() {
        assert_eq!(
            replace_identifier_quotes(r#""col" = 'it''s "fine"'"#),
            r#"[col] = 'it''s "fine"'"#
        );
    }

    #[test]
    fn multiple_identifiers() {
        assert_eq!(
            replace_identifier_quotes(r#""a" = true AND "b" IS NULL"#),
            "[a] = true AND [b] IS NULL"
        );
    }

    #[test]
    fn handles_escaped_double_quotes_in_identifiers() {
        assert_eq!(replace_identifier_quotes(r#""col""name" = 1"#), r#"[col"name] = 1"#);
    }
}
