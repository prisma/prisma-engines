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

use std::{collections::HashMap, marker::PhantomData};

use quaint::{
    ast::{Column, Comparable, ConditionTree, Query, Row, Values},
    visitor::Visitor,
};
use query_builder::{DbQuery, QueryBuilder};
use query_structure::{
    FieldSelection, Filter, Model, ModelProjection, QueryArguments, RecordFilter, SelectionResult, WriteArgs,
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

    fn build_update(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        match selected_fields {
            Some(selected_fields) => {
                let projection = ModelProjection::from(selected_fields);
                let query = update::update_one_with_selection(model, record_filter, args, &projection, &self.context);
                self.convert_query(query)
            }
            None => {
                // this branch is for updates without selections, normally used for databases
                // without RETURNING, the logic is slightly more complicated and will require
                // translating update::update_one_without_selection from the sql-query-connector
                todo!()
            }
        }
    }

    fn build_updates_from_filter(
        &self,
        model: &Model,
        filter: Filter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        let projection = selected_fields.map(ModelProjection::from);
        let query = update::update_many_from_filter(model, filter, args, projection.as_ref(), limit, &self.context);
        Ok(vec![self.convert_query(query)?])
    }

    fn build_delete(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = if let Some(selected_fields) = selected_fields {
            write::delete_returning(model, record_filter.filter, &selected_fields.into(), &self.context)
        } else {
            let mut queries = write::generate_delete_statements(model, record_filter, None, &self.context).into_iter();
            let query = queries.next().expect("should generate at least one query");
            assert_eq!(queries.next(), None, "should generat at most one query");
            query
        };
        self.convert_query(query)
    }

    fn build_deletes(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        let queries = write::generate_delete_statements(model, record_filter, limit, &self.context)
            .into_iter()
            .map(|q| self.convert_query(q))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(queries)
    }

    fn build_raw(
        &self,
        _model: Option<&Model>,
        mut inputs: HashMap<String, prisma_value::PrismaValue>,
        _query_type: Option<String>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        // Unwrapping query & params is safe since it's already passed the query parsing stage
        let query = inputs.remove("query").unwrap().into_string().unwrap();
        let params = inputs.remove("parameters").unwrap().into_list().unwrap();
        Ok(DbQuery::new(query, params))
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
