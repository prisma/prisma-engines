use crate::{
    column_metadata, error::*, model_extensions::*, sql_info::SqlInfo, sql_trace::SqlTraceComment,
    value_ext::IntoTypedJsonExtension, AliasedCondition, ColumnMetadata, SqlRow, ToSqlRow,
};
use async_trait::async_trait;
use connector_interface::{filter::Filter, RecordFilter};
use datamodel::{common::preview_features::PreviewFeature, datamodel_connector::ConnectorCapability};
use futures::future::FutureExt;
use itertools::Itertools;
use opentelemetry::trace::TraceFlags;
use prisma_models::*;
use quaint::{
    ast::*,
    connector::{self, Queryable},
    pooled::PooledConnection,
};
use tracing_futures::Instrument;

use serde_json::{Map, Value};
use std::{collections::HashMap, panic::AssertUnwindSafe};

use crate::sql_trace::trace_parent_to_string;

use opentelemetry::trace::TraceContextExt;
use tracing::{span, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

impl<'t> QueryExt for connector::Transaction<'t> {}
impl QueryExt for PooledConnection {}

/// An extension trait for Quaint's `Queryable`, offering certain Prisma-centric
/// database operations on top of `Queryable`.
#[async_trait]
pub trait QueryExt: Queryable + Send + Sync {
    /// Filter and map the resulting types with the given identifiers.
    #[tracing::instrument(skip(self, q, idents))]
    async fn filter(
        &self,
        q: Query<'_>,
        idents: &[ColumnMetadata<'_>],
        trace_id: Option<String>,
    ) -> crate::Result<Vec<SqlRow>> {
        let span = span!(tracing::Level::INFO, "filter read query");

        let otel_ctx = span.context();
        let span_ref = otel_ctx.span();
        let span_ctx = span_ref.span_context();

        let q = match (q, trace_id) {
            (Query::Select(x), _) if span_ctx.trace_flags() == TraceFlags::SAMPLED => {
                Query::Select(Box::from(x.comment(trace_parent_to_string(span_ctx))))
            }
            // This is part of the required changes to pass a traceid
            (Query::Select(x), Some(traceparent)) => {
                Query::Select(Box::from(x.comment(format!("traceparent={}", traceparent))))
            }
            (q, _) => q,
        };

        let result_set = self.query(q).instrument(span).await?;

        let mut sql_rows = Vec::new();

        for row in result_set {
            sql_rows.push(row.to_sql_row(idents)?);
        }

        Ok(sql_rows)
    }

    /// Execute a singular SQL query in the database, returning an arbitrary
    /// JSON `Value` as a result.
    #[tracing::instrument(skip(self, sql_info, features, inputs))]
    async fn raw_json<'a>(
        &'a self,
        sql_info: SqlInfo,
        features: &[PreviewFeature],
        mut inputs: HashMap<String, PrismaValue>,
    ) -> std::result::Result<Value, crate::error::RawError> {
        // Unwrapping query & params is safe since it's already passed the query parsing stage
        let query = inputs.remove("query").unwrap().into_string().unwrap();
        let params = inputs.remove("parameters").unwrap().into_list().unwrap();
        let params = params.into_iter().map(convert_lossy).collect_vec();
        let result_set = AssertUnwindSafe(self.query_raw(&query, &params))
            .catch_unwind()
            .await??;

        // `query_raw` does not return column names in `ResultSet` when a call to a stored procedure is done
        let columns: Vec<String> = result_set.columns().iter().map(ToString::to_string).collect();
        let mut result = Vec::new();

        for row in result_set.into_iter() {
            let mut object = Map::new();

            for (idx, p_value) in row.into_iter().enumerate() {
                let column_name = columns.get(idx).unwrap_or(&format!("f{}", idx)).clone();
                // TODO: Remove backward_compatible checks for Prisma4
                let backward_compatible = !features.contains(&PreviewFeature::ImprovedQueryRaw)
                    || sql_info.has_capability(ConnectorCapability::BackwardCompatibleQueryRaw);

                object.insert(column_name, p_value.as_typed_json(backward_compatible));
            }

            result.push(Value::Object(object));
        }

        Ok(Value::Array(result))
    }

    /// Execute a singular SQL query in the database, returning the number of
    /// affected rows.
    #[tracing::instrument(skip(self, inputs))]
    async fn raw_count<'a>(
        &'a self,
        mut inputs: HashMap<String, PrismaValue>,
    ) -> std::result::Result<usize, crate::error::RawError> {
        // Unwrapping query & params is safe since it's already passed the query parsing stage
        let query = inputs.remove("query").unwrap().into_string().unwrap();
        let params = inputs.remove("parameters").unwrap().into_list().unwrap();
        let params = params.into_iter().map(convert_lossy).collect_vec();
        let changes = AssertUnwindSafe(self.execute_raw(&query, &params))
            .catch_unwind()
            .await??;

        Ok(changes as usize)
    }

    /// Select one row from the database.
    #[tracing::instrument(skip(self, q, meta))]
    async fn find(
        &self,
        q: Select<'_>,
        meta: &[ColumnMetadata<'_>],
        trace_id: Option<String>,
    ) -> crate::Result<SqlRow> {
        self.filter(q.limit(1).into(), meta, trace_id)
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
        trace_id: Option<String>,
    ) -> crate::Result<Vec<SelectionResult>> {
        if let Some(selectors) = record_filter.selectors {
            Ok(selectors)
        } else {
            self.filter_ids(model, record_filter.filter, trace_id).await
        }
    }

    /// Read the all columns as a (primary) identifier.
    #[tracing::instrument(skip(self, model, filter))]
    async fn filter_ids(
        &self,
        model: &ModelRef,
        filter: Filter,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<SelectionResult>> {
        let model_id: ModelProjection = model.primary_identifier().into();
        let id_cols: Vec<Column<'static>> = model_id.as_columns().collect();

        let select = Select::from_table(model.as_table())
            .columns(id_cols)
            .append_trace(&Span::current())
            .add_trace_id(trace_id.clone())
            .so_that(filter.aliased_cond(None));

        self.select_ids(select, model_id, trace_id).await
    }

    #[tracing::instrument(skip(self, select, model_id))]
    async fn select_ids(
        &self,
        select: Select<'_>,
        model_id: ModelProjection,
        trace_id: Option<String>,
    ) -> crate::Result<Vec<SelectionResult>> {
        let idents: Vec<_> = model_id
            .fields()
            .flat_map(|f| match f {
                Field::Scalar(sf) => vec![sf.type_identifier_with_arity()],
                Field::Relation(rf) => rf.type_identifiers_with_arities(),
                Field::Composite(_) => unimplemented!(),
            })
            .collect();

        let field_names: Vec<_> = model_id.fields().map(|field| field.name()).collect();
        let meta = column_metadata::create(field_names.as_slice(), &idents);

        // TODO: Add tracing
        let mut rows = self.filter(select.into(), &meta, trace_id).await?;
        let mut result = Vec::new();

        for row in rows.drain(0..) {
            let tuples: Vec<_> = model_id.scalar_fields().zip(row.values.into_iter()).collect();
            let record_id: SelectionResult = SelectionResult::new(tuples);

            result.push(record_id);
        }

        Ok(result)
    }
}
