use crate::{column_metadata, error::*, AliasedCondition, ColumnMetadata, SqlRow, ToSqlRow};
use async_trait::async_trait;
use connector_interface::{filter::Filter, RecordFilter};
use futures::future::FutureExt;
use prisma_models::*;
use quaint::{
    ast::*,
    connector::{self, Queryable},
    pooled::PooledConnection,
};
use tracing_futures::Instrument;

use serde_json::{Map, Value};
use std::panic::AssertUnwindSafe;

impl<'t> QueryExt for connector::Transaction<'t> {}
impl QueryExt for PooledConnection {}

/// An extension trait for Quaint's `Queryable`, offering certain Prisma-centric
/// database operations on top of `Queryable`.
#[async_trait]
pub trait QueryExt: Queryable + Send + Sync {
    /// Filter and map the resulting types with the given identifiers.
    #[tracing::instrument(skip(self, q, idents))]
    async fn filter(&self, q: Query<'_>, idents: &[ColumnMetadata<'_>]) -> crate::Result<Vec<SqlRow>> {
        let result_set = self
            .query(q)
            .instrument(tracing::info_span!("Filter read query"))
            .await?;

        let mut sql_rows = Vec::new();

        for row in result_set {
            sql_rows.push(row.to_sql_row(idents)?);
        }

        Ok(sql_rows)
    }

    /// Execute a singular SQL query in the database, returning an arbitrary
    /// JSON `Value` as a result.
    #[tracing::instrument(skip(self, q, params))]
    async fn raw_json<'a>(
        &'a self,
        q: String,
        params: Vec<PrismaValue>,
    ) -> std::result::Result<Value, crate::error::RawError> {
        let params: Vec<_> = params.into_iter().map(convert_lossy).collect();
        let result_set = AssertUnwindSafe(self.query_raw(&q, &params)).catch_unwind().await??;

        let columns: Vec<String> = result_set.columns().iter().map(ToString::to_string).collect();
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
    }

    /// Execute a singular SQL query in the database, returning the number of
    /// affected rows.
    #[tracing::instrument(skip(self, q, params))]
    async fn raw_count<'a>(
        &'a self,
        q: String,
        params: Vec<PrismaValue>,
    ) -> std::result::Result<usize, crate::error::RawError> {
        let params: Vec<_> = params.into_iter().map(convert_lossy).collect();
        let changes = AssertUnwindSafe(self.execute_raw(&q, &params)).catch_unwind().await??;

        Ok(changes as usize)
    }

    /// Select one row from the database.
    #[tracing::instrument(skip(self, q, meta))]
    async fn find(&self, q: Select<'_>, meta: &[ColumnMetadata<'_>]) -> crate::Result<SqlRow> {
        self.filter(q.limit(1).into(), meta)
            .await?
            .into_iter()
            .next()
            .ok_or(SqlError::RecordDoesNotExist)
    }

    /// Process the record filter and either return directly with precomputed values,
    /// or fetch IDs from the database.
    #[tracing::instrument(skip(self, model, record_filter))]
    async fn filter_selectors(
        &self,
        model: &ModelRef,
        record_filter: RecordFilter,
    ) -> crate::Result<Vec<RecordProjection>> {
        if let Some(selectors) = record_filter.selectors {
            Ok(selectors)
        } else {
            self.filter_ids(model, record_filter.filter).await
        }
    }

    /// Read the all columns as a (primary) identifier.
    #[tracing::instrument(skip(self, model, filter))]
    async fn filter_ids(&self, model: &ModelRef, filter: Filter) -> crate::Result<Vec<RecordProjection>> {
        let model_id = model.primary_identifier();
        let id_cols: Vec<Column<'static>> = model_id.as_columns().collect();

        let select = Select::from_table(model.as_table())
            .columns(id_cols)
            .so_that(filter.aliased_cond(None));

        self.select_ids(select, model_id).await
    }

    #[tracing::instrument(skip(self, select, model_id))]
    async fn select_ids(&self, select: Select<'_>, model_id: ModelProjection) -> crate::Result<Vec<RecordProjection>> {
        let idents: Vec<_> = model_id
            .fields()
            .flat_map(|f| match f {
                Field::Scalar(sf) => vec![sf.type_identifier_with_arity()],
                Field::Relation(rf) => rf.type_identifiers_with_arities(),
            })
            .collect();

        let field_names: Vec<_> = model_id.fields().map(|field| field.name()).collect();
        let meta = column_metadata::create(field_names.as_slice(), &idents);

        let mut rows = self.filter(select.into(), &meta).await?;
        let mut result = Vec::new();

        for row in rows.drain(0..) {
            let tuples: Vec<_> = model_id.scalar_fields().zip(row.values.into_iter()).collect();
            let record_id: RecordProjection = RecordProjection::new(tuples);

            result.push(record_id);
        }

        Ok(result)
    }
}
