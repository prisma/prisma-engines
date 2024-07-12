use crate::filter::FilterBuilder;
use crate::ser_raw::SerializedResultSet;
use crate::{
    column_metadata, error::*, model_extensions::*, sql_trace::trace_parent_to_string, sql_trace::SqlTraceComment,
    ColumnMetadata, Context, SqlRow, ToSqlRow,
};
use async_trait::async_trait;
use connector_interface::RecordFilter;
use futures::future::FutureExt;
use itertools::Itertools;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::TraceFlags;
use quaint::{ast::*, connector::Queryable};
use query_structure::*;
use std::{collections::HashMap, panic::AssertUnwindSafe};
use tracing::{info_span, Span};
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[async_trait]
impl<Q: Queryable + ?Sized> QueryExt for Q {
    async fn filter(
        &self,
        q: Query<'_>,
        idents: &[ColumnMetadata<'_>],
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SqlRow>> {
        let span = info_span!("filter read query");

        let otel_ctx = span.context();
        let span_ref = otel_ctx.span();
        let span_ctx = span_ref.span_context();

        let q = match (q, ctx.trace_id) {
            (Query::Select(x), _) if span_ctx.trace_flags() == TraceFlags::SAMPLED => {
                Query::Select(Box::from(x.comment(trace_parent_to_string(span_ctx))))
            }
            // This is part of the required changes to pass a traceid
            (Query::Select(x), trace_id) => Query::Select(Box::from(x.add_trace_id(trace_id))),
            (q, _) => q,
        };

        let result_set = self.query(q).instrument(span).await?;

        let mut sql_rows = Vec::new();

        for row in result_set {
            sql_rows.push(row.to_sql_row(idents)?);
        }

        Ok(sql_rows)
    }

    async fn raw_json<'a>(
        &'a self,
        mut inputs: HashMap<String, PrismaValue>,
    ) -> std::result::Result<RawJson, crate::error::RawError> {
        // Unwrapping query & params is safe since it's already passed the query parsing stage
        let query = inputs.remove("query").unwrap().into_string().unwrap();
        let params = inputs.remove("parameters").unwrap().into_list().unwrap();
        let params = params.into_iter().map(convert_lossy).collect_vec();
        let result_set = AssertUnwindSafe(self.query_raw_typed(&query, &params))
            .catch_unwind()
            .await??;
        let raw_json = RawJson::try_new(SerializedResultSet(result_set))?;

        Ok(raw_json)
    }

    async fn raw_count<'a>(
        &'a self,
        mut inputs: HashMap<String, PrismaValue>,
        _features: psl::PreviewFeatures,
    ) -> std::result::Result<usize, crate::error::RawError> {
        // Unwrapping query & params is safe since it's already passed the query parsing stage
        let query = inputs.remove("query").unwrap().into_string().unwrap();
        let params = inputs.remove("parameters").unwrap().into_list().unwrap();
        let params = params.into_iter().map(convert_lossy).collect_vec();
        let changes = AssertUnwindSafe(self.execute_raw_typed(&query, &params))
            .catch_unwind()
            .await??;

        Ok(changes as usize)
    }

    async fn find(&self, q: Select<'_>, meta: &[ColumnMetadata<'_>], ctx: &Context<'_>) -> crate::Result<SqlRow> {
        self.filter(q.limit(1).into(), meta, ctx)
            .await?
            .into_iter()
            .next()
            .ok_or(SqlError::RecordDoesNotExist {
                cause: "Filter returned no results".to_owned(),
            })
    }

    async fn filter_selectors(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SelectionResult>> {
        if let Some(selectors) = record_filter.selectors {
            Ok(selectors)
        } else {
            self.filter_ids(model, record_filter.filter, ctx).await
        }
    }

    async fn filter_ids(
        &self,
        model: &Model,
        filter: Filter,
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SelectionResult>> {
        let model_id: ModelProjection = model.primary_identifier().into();
        let id_cols: Vec<Column<'static>> = model_id.as_columns(ctx).collect();
        let condition = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

        let select = Select::from_table(model.as_table(ctx))
            .columns(id_cols)
            .append_trace(&Span::current())
            .add_trace_id(ctx.trace_id)
            .so_that(condition);

        self.select_ids(select, model_id, ctx).await
    }

    async fn select_ids(
        &self,
        select: Select<'_>,
        model_id: ModelProjection,
        ctx: &Context<'_>,
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
        let mut rows = self.filter(select.into(), &meta, ctx).await?;
        let mut result = Vec::new();

        for row in rows.drain(0..) {
            let tuples: Vec<_> = model_id.scalar_fields().zip(row.values.into_iter()).collect();
            let record_id: SelectionResult = SelectionResult::new(tuples);

            result.push(record_id);
        }

        Ok(result)
    }
}

/// An extension trait for Quaint's `Queryable`, offering certain Prisma-centric
/// database operations on top of `Queryable`.
#[async_trait]
pub(crate) trait QueryExt {
    /// Filter and map the resulting types with the given identifiers.
    async fn filter(
        &self,
        q: Query<'_>,
        idents: &[ColumnMetadata<'_>],
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SqlRow>>;

    /// Execute a singular SQL query in the database, returning an arbitrary
    /// JSON `Value` as a result.
    async fn raw_json<'a>(
        &'a self,
        mut inputs: HashMap<String, PrismaValue>,
    ) -> std::result::Result<RawJson, crate::error::RawError>;

    /// Execute a singular SQL query in the database, returning the number of
    /// affected rows.
    async fn raw_count<'a>(
        &'a self,
        mut inputs: HashMap<String, PrismaValue>,
        _features: psl::PreviewFeatures,
    ) -> std::result::Result<usize, crate::error::RawError>;

    /// Select one row from the database.
    async fn find(&self, q: Select<'_>, meta: &[ColumnMetadata<'_>], ctx: &Context<'_>) -> crate::Result<SqlRow>;

    /// Process the record filter and either return directly with precomputed values,
    /// or fetch IDs from the database.
    async fn filter_selectors(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SelectionResult>>;

    /// Read the all columns as a (primary) identifier.
    async fn filter_ids(&self, model: &Model, filter: Filter, ctx: &Context<'_>)
        -> crate::Result<Vec<SelectionResult>>;

    async fn select_ids(
        &self,
        select: Select<'_>,
        model_id: ModelProjection,
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SelectionResult>>;
}
