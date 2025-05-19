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
pub mod value;
pub mod write;

use std::{collections::HashMap, marker::PhantomData};

use itertools::{Either, Itertools};
use prisma_value::PrismaValue;
use quaint::{
    ast::{Column, Comparable, ConditionTree, ExpressionKind, Insert, OnConflict, OpaqueType, Query, Row, Values},
    visitor::Visitor,
    Value,
};
use query_builder::{DbQuery, QueryBuilder};
use query_structure::{
    AggregationSelection, FieldSelection, Filter, Model, ModelProjection, QueryArguments, RecordFilter, RelationField,
    ScalarField, SelectionResult, WriteArgs,
};

pub use column_metadata::ColumnMetadata;
pub use context::Context;
pub use convert::opaque_type_to_prisma_type;
pub use filter::FilterBuilder;
pub use model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt, SelectionResultExt};
use read::alias_with_db_name;
pub use sql_trace::SqlTraceComment;
use value::GeneratorCall;

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
        let template = V::build_template(query)?;

        let params = template
            .parameters
            .into_iter()
            .map(|v| convert::quaint_value_to_prisma_value(v, self.context.sql_family()))
            .collect::<Vec<_>>();

        Ok(DbQuery::TemplateSql {
            fragments: template.fragments,
            placeholder_format: template.placeholder_format,
            params,
        })
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

    #[cfg(feature = "relation_joins")]
    fn build_get_related_records(
        &self,
        join_links: query_builder::JoinLinks,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        use std::slice;

        use filter::default_scalar_filter;
        use itertools::Itertools;
        use quaint::ast::{Aliasable, Joinable, Select};
        use select::{JoinConditionExt, SelectBuilderExt};

        let link_alias = join_links.to_string();
        let (rf, conditions_per_field) = join_links.into_parent_field_and_conditions();

        let m2m_alias = self.context.next_table_alias();
        let m2m_table = rf.as_table(&self.context).alias(m2m_alias.to_string());

        let related_alias = self.context.next_table_alias();
        let related_table = rf
            .related_model()
            .as_table(&self.context)
            .alias(related_alias.to_string());

        let m2m_col = rf
            .related_field()
            .m2m_column(&self.context)
            .table(m2m_alias.to_string());

        let left_scalar = rf
            .related_field()
            .left_scalars()
            .into_iter()
            .exactly_one()
            .expect("should have one left scalar in m2m relation");
        let (_, conditions) = conditions_per_field
            .exactly_one()
            .expect("should have one field in m2m relation");

        let filter = conditions
            .into_iter()
            .map(|cond| {
                default_scalar_filter(
                    m2m_col.clone().into(),
                    cond,
                    slice::from_ref(&left_scalar),
                    None,
                    &self.context,
                )
            })
            .reduce(|l, r| l.and(r));

        let columns = ModelProjection::from(selected_fields)
            .as_columns(&self.context)
            .map(|col| col.table(related_alias.to_string()))
            // Add an m2m column with an alias to make it possible to join it outside of this
            // function.
            .chain([m2m_col.alias(link_alias)]);

        let join_condition = rf.m2m_join_conditions(Some(m2m_alias), Some(related_alias), &self.context);

        let select = Select::from_table(m2m_table)
            .columns(columns)
            .inner_join(related_table.on(join_condition))
            .with_distinct(&query_arguments, related_alias)
            .with_ordering(&query_arguments, Some(related_alias.to_string()), &self.context)
            .with_pagination(&query_arguments, None)
            .with_filters(query_arguments.filter, Some(related_alias), &self.context);

        let select = if let Some(filter) = filter {
            select.and_where(filter)
        } else {
            select
        };

        self.convert_query(select)
    }

    fn build_aggregate(
        &self,
        model: &Model,
        args: QueryArguments,
        selections: &[AggregationSelection],
        group_by: Vec<ScalarField>,
        having: Option<Filter>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = if group_by.is_empty() {
            read::aggregate(model, selections, args, alias_with_db_name(), &self.context)
        } else {
            read::group_by_aggregate(
                model,
                args,
                selections,
                group_by,
                having,
                alias_with_db_name(),
                &self.context,
            )
        };
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

    fn build_updates(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        let projection = selected_fields.map(ModelProjection::from);
        write::generate_update_statements(model, record_filter, args, projection.as_ref(), limit, &self.context)
            .into_iter()
            .map(|query| self.convert_query(query))
            .collect::<Result<Vec<_>, _>>()
    }

    fn build_upsert(
        &self,
        model: &Model,
        filter: Filter,
        create_args: WriteArgs,
        update_args: WriteArgs,
        selected_fields: &FieldSelection,
        unique_constraints: &[ScalarField],
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = write::native_upsert(
            model,
            filter,
            create_args,
            update_args,
            &selected_fields.into(),
            unique_constraints,
            &self.context,
        );
        self.convert_query(query)
    }

    fn build_m2m_connect(
        &self,
        field: RelationField,
        parent: PrismaValue,
        child: PrismaValue,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let relation = field.relation();

        let parent_column = field.related_field().m2m_column(&self.context);
        let child_column = field.m2m_column(&self.context);

        // parent and child can refer to arrays, so we need a product of the two
        let call = GeneratorCall::new("product", vec![parent, child]);
        let insert = Insert::expression_into(
            relation.as_table(&self.context),
            vec![parent_column, child_column],
            ExpressionKind::Parameterized(Value::opaque(call, OpaqueType::Unknown)),
        );
        let query = insert.on_conflict(OnConflict::DoNothing);
        self.convert_query(query)
    }

    fn build_m2m_disconnect(
        &self,
        field: RelationField,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        let query = write::delete_relation_table_records(&field, parent_id, child_ids, &self.context);
        self.convert_query(query)
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
            write::generate_delete_statements(model, record_filter, None, &self.context)
                .into_iter()
                .exactly_one()
                .expect("should generate exactly one delete")
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
        Ok(DbQuery::RawSql { sql: query, params })
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
    let iter = match results.into_iter().exactly_one() {
        Ok(result) if matches!(&result.pairs[..], [(_, PrismaValue::Placeholder { .. })]) => {
            return Row::from(columns.to_vec())
                .in_selection(ExpressionKind::ParameterizedRow(result.db_values(ctx).pop().unwrap()))
                .into()
        }
        // fall back to the default behavior in other cases
        Ok(item) => Either::Left(std::iter::once(item)),
        Err(items) => Either::Right(items),
    };
    let mut values = Values::empty();

    for result in iter {
        let vals: Vec<_> = result.db_values(ctx);
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
