use std::ops::ControlFlow;

use log::debug;
use serde::{Deserialize, Serialize};
use sqlparser::{
    ast::{VisitMut, VisitorMut},
    dialect::GenericDialect,
    parser::Parser,
};

pub type RawQuery = String;
pub type PrismaQuery = String;
pub type QueryPlan = String;
pub type Tag = String;

#[derive(Deserialize, Serialize, Debug)]
pub struct SubmittedQueryInfo {
    pub raw_query: RawQuery,
    pub tag: Tag,
    pub prisma_query: PrismaQuery,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlowQuery {
    pub sql: RawQuery,
    pub prisma_queries: Vec<PrismaQuery>,
    pub mean_exec_time: f64,
    pub num_executions: i64,
    pub query_plan: QueryPlan,
    pub additional_info: serde_json::Value,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Deserialize, Serialize)]
pub struct RawQueryShape(pub String);

impl RawQueryShape {
    pub fn from_raw_query(sql: &str) -> Self {
        let dialect = GenericDialect {}; // or AnsiDialect, or your own dialect ...
        let mut ast = Parser::parse_sql(&dialect, sql).unwrap();
        ast.visit(&mut ValueRewriter);

        let shape = ast
            .iter_mut()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        debug!("transforming sql into shape: ({}) -> ({})", sql, shape);
        RawQueryShape(shape)
    }
}

struct ValueRewriter;

impl VisitorMut for ValueRewriter {
    type Break = ();

    fn post_visit_expr(&mut self, expr: &mut sqlparser::ast::Expr) -> std::ops::ControlFlow<Self::Break> {
        if let sqlparser::ast::Expr::Value(_) = expr {
            *expr = sqlparser::ast::Expr::Value(sqlparser::ast::Value::Placeholder("?".to_string()));
        }
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_shape() {
        for s in vec![
            "SELECT * FROM foo WHERE bar = 1",
            "Select * From foo where bar = 'wadus'",
        ] {
            assert_eq!(
                RawQueryShape("SELECT * FROM foo WHERE bar = ?".to_string()),
                RawQueryShape::from_raw_query(s)
            );
        }
    }
}
