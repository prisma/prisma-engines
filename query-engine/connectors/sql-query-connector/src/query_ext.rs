use crate::{error::*, AliasedCondition, RawQuery, SqlRow, ToSqlRow};
use async_trait::async_trait;
use connector_interface::filter::Filter;
use datamodel::FieldArity;
use prisma_models::*;
use quaint::{
    ast::*,
    connector::{self, Queryable},
    pooled::PooledConnection,
};
use serde_json::{Map, Number, Value};
use std::convert::TryFrom;

impl<'t> QueryExt for connector::Transaction<'t> {}
impl QueryExt for PooledConnection {}

/// Functions for querying data.
/// Basically represents a connection wrapper?
#[async_trait]
pub trait QueryExt: Queryable + Send + Sync {
    async fn filter(&self, q: Query<'_>, idents: &[(TypeIdentifier, FieldArity)]) -> crate::Result<Vec<SqlRow>> {
        let result_set = self.query(q).await?;
        let mut sql_rows = Vec::new();

        for row in result_set {
            sql_rows.push(row.to_sql_row(idents)?);
        }

        Ok(sql_rows)
    }

    async fn raw_json(&self, q: RawQuery) -> crate::Result<Value> {
        if q.is_select() {
            let result_set = self.query_raw(q.0.as_str(), &[]).await?;
            let columns: Vec<String> = result_set.columns().map(ToString::to_string).collect();
            let mut result = Vec::new();

            for row in result_set.into_iter() {
                let mut object = Map::new();

                for (idx, p_value) in row.into_iter().enumerate() {
                    let column_name: String = columns[idx].clone();
                    object.insert(column_name, Value::from(p_value));
                }

                result.push(Value::Object(object));
            }

            Ok(Value::Array(result))
        } else {
            let changes = self.execute_raw(q.0.as_str(), &[]).await?;
            Ok(Value::Number(Number::from(changes)))
        }
    }

    /// Select one row from the database.
    async fn find(&self, q: Select<'_>, idents: &[(TypeIdentifier, FieldArity)]) -> crate::Result<SqlRow> {
        self.filter(q.limit(1).into(), idents)
            .await?
            .into_iter()
            .next()
            .ok_or(SqlError::RecordDoesNotExist)
    }

    /// Read the first column from the first row as an integer.
    async fn find_int(&self, q: Select<'_>) -> crate::Result<i64> {
        // UNWRAP: A dataset will always have at least one column, even if it contains no data.
        let id = self
            .find(q, &[(TypeIdentifier::Int, FieldArity::Required)])
            .await?
            .values
            .into_iter()
            .next()
            .unwrap();

        Ok(i64::try_from(id)?)
    }

    /// Read the all columns as an `GraphqlId`
    async fn filter_ids(&self, model: &ModelRef, filter: Filter) -> crate::Result<Vec<GraphqlId>> {
        let select = Select::from_table(model.as_table())
            .column(model.fields().id().as_column())
            .so_that(filter.aliased_cond(None));

        self.select_ids(select).await
    }

    async fn select_ids(&self, select: Select<'_>) -> crate::Result<Vec<GraphqlId>> {
        let mut rows = self
            .filter(select.into(), &[(TypeIdentifier::GraphQLID, FieldArity::Required)])
            .await?;
        let mut result = Vec::new();

        for mut row in rows.drain(0..) {
            for value in row.values.drain(0..) {
                result.push(GraphqlId::try_from(value)?)
            }
        }

        Ok(result)
    }
}
