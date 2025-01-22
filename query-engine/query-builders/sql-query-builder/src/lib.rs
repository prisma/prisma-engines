pub mod column_metadata;
mod context;
mod convert;
mod cursor_condition;
mod filter;
mod join_utils;
pub mod limit;
mod model_extensions;
mod nested_aggregations;
mod ordering;
pub mod read;
#[cfg(feature = "relation_joins")]
pub mod select;
mod sql_trace;
pub mod update;
pub mod write;

use std::marker::PhantomData;

use quaint::{
    ast::{Column, Comparable, ConditionTree, Query, Row, Values},
    visitor::Visitor,
};
use query_builder::{DbQuery, QueryBuilder};
use query_structure::{
    FieldSelection, Model, ModelProjection, QueryArguments, RecordFilter, SelectionResult, WriteArgs,
};

pub use column_metadata::ColumnMetadata;
pub use context::Context;
pub use filter::FilterBuilder;
pub use model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt, SelectionResultExt};
pub use sql_trace::SqlTraceComment;

const PARAMETER_LIMIT: usize = 2000;

pub struct SqlQueryBuilder<'a, Visitor> {
    context: Context<'a>,
    phantom: PhantomData<fn(Visitor)>,
}

impl<'a, V> SqlQueryBuilder<'a, V> {
    pub fn new(context: Context<'a>) -> Self {
        Self {
            context,
            phantom: PhantomData,
        }
    }

    fn convert_query(&self, query: impl Into<Query<'a>>) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>
    where
        V: Visitor<'a>,
    {
        let (sql, params) = V::build(query)?;
        let params = params
            .into_iter()
            .map(convert::quaint_value_to_prisma_value)
            .collect::<Vec<_>>();
        Ok(DbQuery::new(sql, params))
    }
}

impl<'a, V: Visitor<'a>> QueryBuilder for SqlQueryBuilder<'a, V> {
    fn build_get_records(
        &self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = read::get_records(
            model,
            ModelProjection::from(selected_fields)
                .as_columns(&self.context)
                .mark_all_selected(),
            selected_fields.virtuals(),
            query_arguments,
            &self.context,
        );
        self.convert_query(query)
    }

    fn build_create_record(
        &self,
        model: &Model,
        args: WriteArgs,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = write::create_record(model, args, &selected_fields.into(), &self.context);
        self.convert_query(query)
    }

    fn build_inserts(
        &self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        let projection = selected_fields.map(ModelProjection::from);
        let query = write::generate_insert_statements(model, args, skip_duplicates, projection.as_ref(), &self.context);
        query.into_iter().map(|q| self.convert_query(q)).collect()
    }

    fn build_updates_from_filter(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        let projection = selected_fields.map(ModelProjection::from);
        assert!(
            !record_filter.has_selectors(),
            "build_updates_from_filter cannot be called with selectors"
        );
        let query =
            update::update_many_from_filter(model, record_filter, args, projection.as_ref(), limit, &self.context);
        Ok(vec![self.convert_query(query)?])
    }
}

pub fn chunked_conditions<F, Q>(
    columns: &[Column<'static>],
    records: &[SelectionResult],
    ctx: &Context<'_>,
    f: F,
) -> Vec<Query<'static>>
where
    Q: Into<Query<'static>>,
    F: Fn(ConditionTree<'static>) -> Q,
{
    records
        .chunks(PARAMETER_LIMIT)
        .map(|chunk| {
            let tree = in_conditions(columns, chunk, ctx);
            f(tree).into()
        })
        .collect()
}

pub fn in_conditions<'a>(
    columns: &'a [Column<'static>],
    results: impl IntoIterator<Item = &'a SelectionResult>,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    let mut values = Values::empty();

    for result in results.into_iter() {
        let vals: Vec<_> = result.db_values(ctx);
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
