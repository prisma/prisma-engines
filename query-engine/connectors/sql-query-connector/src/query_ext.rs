use crate::ser_raw::SerializedResultSet;
use crate::{SqlRow, ToSqlRow, error::*};
use async_trait::async_trait;
use futures::future::FutureExt;
use itertools::Itertools;
use prisma_value::Placeholder as PrismaValuePlaceholder;
use quaint::{ast::*, connector::Queryable};
use query_structure::*;
use sql_query_builder::value::{GeneratorCall, Placeholder};
use sql_query_builder::{AsColumns, AsTable, ColumnMetadata, Context, FilterBuilder, SqlTraceComment, column_metadata};
use std::{collections::HashMap, panic::AssertUnwindSafe};
use tracing::info_span;
use tracing_futures::Instrument;

#[async_trait]
impl<Q: Queryable + ?Sized> QueryExt for Q {
    async fn filter(
        &self,
        q: Query<'_>,
        idents: &[ColumnMetadata<'_>],
        ctx: &Context<'_>,
    ) -> crate::Result<Vec<SqlRow>> {
        let span = info_span!("prisma:engine:filter_read_query");

        let q = match q {
            Query::Select(x) => Query::Select(Box::from(x.add_traceparent(ctx.traceparent()))),
            q => q,
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
        let params = params
            .into_iter()
            .map(convert_prisma_value_to_quaint_lossy)
            .collect_vec();
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
        let params = params
            .into_iter()
            .map(convert_prisma_value_to_quaint_lossy)
            .collect_vec();
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
        let model_id: ModelProjection = model.shard_aware_primary_identifier().into();
        let id_cols: Vec<Column<'static>> = model_id.as_columns(ctx).collect();
        let condition = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

        let select = Select::from_table(model.as_table(ctx))
            .columns(id_cols)
            .add_traceparent(ctx.traceparent())
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

        let rows = self.filter(select.into(), &meta, ctx).await?;
        let result = rows
            .into_iter()
            .map(|row| {
                let tuples = model_id.scalar_fields().zip(row.values.into_iter()).collect();
                SelectionResult::new(tuples)
            })
            .collect();

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

/// Attempts to convert a PrismaValue to a database value without any additional type information.
/// Can't reliably map Null values.
pub fn convert_prisma_value_to_quaint_lossy<'a>(pv: PrismaValue) -> Value<'a> {
    match pv {
        PrismaValue::String(s) => s.into(),
        PrismaValue::Float(f) => f.into(),
        PrismaValue::Boolean(b) => b.into(),
        PrismaValue::DateTime(d) => d.with_timezone(&chrono::Utc).into(),
        PrismaValue::Enum(e) => e.into(),
        PrismaValue::Int(i) => i.into(),
        PrismaValue::BigInt(i) => i.into(),
        PrismaValue::Uuid(u) => u.to_string().into(),
        PrismaValue::List(l) => Value::array(l.into_iter().map(convert_prisma_value_to_quaint_lossy)),
        PrismaValue::Json(s) => Value::json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Bytes(b) => Value::bytes(b),
        PrismaValue::Null => Value::null_int32(), // Can't tell which type the null is supposed to be.
        PrismaValue::Object(_) => unimplemented!(),
        PrismaValue::Placeholder(PrismaValuePlaceholder { name, r#type }) => {
            Value::opaque(Placeholder::new(name), convert_prisma_type_to_opaque_type(&r#type))
        }
        PrismaValue::GeneratorCall {
            name,
            args,
            return_type,
        } => Value::opaque(
            GeneratorCall::new(name, args),
            convert_prisma_type_to_opaque_type(&return_type),
        ),
    }
}

pub fn convert_prisma_type_to_opaque_type(pt: &PrismaValueType) -> OpaqueType {
    match pt {
        PrismaValueType::Any => OpaqueType::Unknown,
        PrismaValueType::String => OpaqueType::Text,
        PrismaValueType::Uuid => OpaqueType::Uuid,
        PrismaValueType::Int => OpaqueType::Int32,
        PrismaValueType::BigInt => OpaqueType::Int64,
        PrismaValueType::Float => OpaqueType::Numeric,
        PrismaValueType::Boolean => OpaqueType::Boolean,
        PrismaValueType::DateTime => OpaqueType::DateTime,
        PrismaValueType::List(t) => OpaqueType::Array(Box::new(convert_prisma_type_to_opaque_type(t))),
        PrismaValueType::Json => OpaqueType::Json,
        PrismaValueType::Object => OpaqueType::Object,
        PrismaValueType::Bytes => OpaqueType::Bytes,
        PrismaValueType::Enum => OpaqueType::Text,
    }
}
