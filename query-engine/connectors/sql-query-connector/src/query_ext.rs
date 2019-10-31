use crate::{error::*, query_builder::ReadQueryBuilder, AliasedCondition, RawQuery, SqlRow, ToSqlRow};
use connector_interface::{
    error::RecordFinderInfo,
    filter::{Filter, RecordFinder},
};
use futures::future::{BoxFuture, FutureExt};
use prisma_models::*;
use prisma_query::{
    ast::*,
    connector::{self, Queryable},
};
use serde_json::{Map, Number, Value};
use std::{convert::TryFrom, sync::Arc};

impl<'t> QueryExt for connector::Transaction<'t> {}

/// Functions for querying data.
/// Basically represents a connection wrapper?
pub trait QueryExt: Queryable {
    fn filter<'a>(&'a self, q: Query<'a>, idents: &'a [TypeIdentifier]) -> BoxFuture<'a, crate::Result<Vec<SqlRow>>> {
        let fut = async move {
            let result_set = self.query(q).await?;
            let mut sql_rows = Vec::new();

            for row in result_set {
                sql_rows.push(row.to_sql_row(idents)?);
            }

            Ok(sql_rows)
        };

        fut.boxed()
    }

    fn raw_json<'a>(&'a self, q: RawQuery) -> BoxFuture<'a, crate::Result<Value>> {
        let fut = async move {
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
        };

        fut.boxed()
    }

    /// Find one full record selecting all scalar fields.
    fn find_record<'a>(&'a self, record_finder: &'a RecordFinder) -> BoxFuture<'a, crate::Result<SingleRecord>> {
        use SqlError::*;

        let fut = async move {
            let model = record_finder.field.model();
            let selected_fields = SelectedFields::from(Arc::clone(&model));
            let select = ReadQueryBuilder::get_records(model, &selected_fields, record_finder);
            let idents = selected_fields.type_identifiers();

            let row = self.find(select, idents.as_slice()).await.map_err(|e| match e {
                RecordDoesNotExist => RecordNotFoundForWhere(RecordFinderInfo::from(record_finder)),
                e => e,
            })?;

            let record = Record::from(row);

            Ok(SingleRecord::new(record, selected_fields.names()))
        };

        fut.boxed()
    }

    /// Select one row from the database.
    fn find<'a>(&'a self, q: Select<'a>, idents: &'a [TypeIdentifier]) -> BoxFuture<'a, crate::Result<SqlRow>> {
        let fut = async move {
            self.filter(q.limit(1).into(), idents)
                .await?
                .into_iter()
                .next()
                .ok_or(SqlError::RecordDoesNotExist)
        };

        fut.boxed()
    }

    /// Read the first column from the first row as an integer.
    fn find_int<'a>(&'a self, q: Select<'a>) -> BoxFuture<'a, crate::Result<i64>> {
        let fut = async move {
            // UNWRAP: A dataset will always have at least one column, even if it contains no data.
            let id = self
                .find(q, &[TypeIdentifier::Int])
                .await?
                .values
                .into_iter()
                .next()
                .unwrap();

            Ok(i64::try_from(id)?)
        };

        fut.boxed()
    }

    /// Read the first column from the first row as an `GraphqlId`.
    fn find_id<'a>(&'a self, record_finder: &'a RecordFinder) -> BoxFuture<'a, crate::Result<GraphqlId>> {
        let fut = async move {
            let model = record_finder.field.model();
            let filter = Filter::from(record_finder.clone());

            let id = self
                .filter_ids(model, filter)
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| SqlError::RecordNotFoundForWhere(RecordFinderInfo::from(record_finder)))?;

            Ok(id)
        };

        fut.boxed()
    }

    /// Read the all columns as an `GraphqlId`
    fn filter_ids<'a>(&'a self, model: ModelRef, filter: Filter) -> BoxFuture<'a, crate::Result<Vec<GraphqlId>>> {
        let select = Select::from_table(model.table())
            .column(model.fields().id().as_column())
            .so_that(filter.aliased_cond(None));

        self.select_ids(select)
    }

    fn select_ids<'a>(&'a self, select: Select<'a>) -> BoxFuture<'a, crate::Result<Vec<GraphqlId>>> {
        let fut = async move {
            let mut rows = self.filter(select.into(), &[TypeIdentifier::GraphQLID]).await?;
            let mut result = Vec::new();

            for mut row in rows.drain(0..) {
                for value in row.values.drain(0..) {
                    result.push(GraphqlId::try_from(value)?)
                }
            }

            Ok(result)
        };

        fut.boxed()
    }

    /// Find a child of a parent. Will return an error if no child found with
    /// the given parameters. A more restrictive version of `get_ids_by_parents`.
    fn find_id_by_parent<'a>(
        &'a self,
        parent_field: RelationFieldRef,
        parent_id: &'a GraphqlId,
        selector: &'a Option<RecordFinder>,
    ) -> BoxFuture<'a, crate::Result<GraphqlId>> {
        let fut = async move {
            let ids = self
                .filter_ids_by_parents(
                    Arc::clone(&parent_field),
                    vec![parent_id],
                    selector.clone().map(Filter::from),
                )
                .await?;

            let id = ids.into_iter().next().ok_or_else(|| SqlError::RecordsNotConnected {
                relation_name: parent_field.relation().name.clone(),
                parent_name: parent_field.model().name.clone(),
                parent_where: None,
                child_name: parent_field.related_model().name.clone(),
                child_where: selector.as_ref().map(RecordFinderInfo::from).map(Box::new),
            })?;

            Ok(id)
        };

        fut.boxed()
    }

    /// Find all children record id's with the given parent id's, optionally given
    /// a `Filter` for extra filtering.
    fn filter_ids_by_parents<'a>(
        &'a self,
        parent_field: RelationFieldRef,
        parent_ids: Vec<&'a GraphqlId>,
        selector: Option<Filter>,
    ) -> BoxFuture<'a, crate::Result<Vec<GraphqlId>>> {
        let related_model = parent_field.related_model();
        let relation = parent_field.relation();
        let child_id_field = relation.column_for_relation_side(parent_field.relation_side.opposite());
        let parent_id_field = relation.column_for_relation_side(parent_field.relation_side);

        let subselect = Select::from_table(relation.relation_table())
            .column(child_id_field)
            .so_that(parent_id_field.in_selection(parent_ids));

        let conditions = related_model
            .fields()
            .id()
            .db_name()
            .to_string()
            .in_selection(subselect);

        let conditions = match selector {
            Some(into_cond) => {
                let filter: Filter = into_cond.into();
                conditions.and(filter.aliased_cond(None))
            }
            None => conditions.into(),
        };

        let select = Select::from_table(related_model.table())
            .column(related_model.fields().id().as_column())
            .so_that(conditions);

        self.select_ids(select)
    }
}
