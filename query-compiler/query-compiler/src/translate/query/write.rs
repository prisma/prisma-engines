use itertools::{Either, Itertools};
use query_builder::QueryBuilder;
use query_core::{
    ConnectRecords, DeleteManyRecords, DeleteRecord, DisconnectRecords, RawQuery, UpdateManyRecords, UpdateRecord,
    UpdateRecordWithSelection, WriteQuery,
};
use query_structure::{QueryArguments, RelationLoadStrategy, Take};
use sql_query_builder::write::split_write_args_by_shape;
use thiserror::Error;

use crate::{TranslateError, expression::Expression, translate::TranslateResult};

use super::read::add_inmemory_join;

pub(crate) fn translate_write_query(query: WriteQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        WriteQuery::CreateRecord(cr) => {
            // TODO: MySQL needs additional logic to generate IDs on our side.
            // See sql_query_connector::database::operations::write::create_record
            let query = builder
                .build_create_record(&cr.model, cr.args, &cr.selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;

            // TODO: we probably need some additional node type or extra info in the WriteQuery node
            // to help the client executor figure out the returned ID in the case when it's inferred
            // from the query arguments.
            Expression::Unique(Box::new(Expression::Query(query)))
        }

        WriteQuery::CreateManyRecords(cmr) => {
            let split_args = if cmr.split_by_shape && !cmr.args.is_empty() {
                Either::Left(split_write_args_by_shape(&cmr.model, cmr.args))
            } else {
                Either::Right([cmr.args])
            };
            let (projection, nested) = cmr.selected_fields.map(|sf| (sf.fields, sf.nested)).unzip();

            let inserts = split_args
                .into_iter()
                .map(|args| {
                    builder
                        .build_inserts(&cmr.model, args, cmr.skip_duplicates, projection.as_ref())
                        .map_err(TranslateError::QueryBuildFailure)
                })
                .flatten_ok();

            if projection.is_some() {
                let mut expr = Expression::Concat(inserts.map_ok(Expression::Query).try_collect()?);
                let nested = nested.unwrap_or_default();
                if !nested.is_empty() {
                    expr = add_inmemory_join(expr, nested, builder)?;
                }
                expr
            } else {
                Expression::Sum(inserts.map_ok(Expression::Execute).try_collect()?)
            }
        }

        WriteQuery::UpdateManyRecords(UpdateManyRecords {
            name: _,
            model,
            record_filter,
            args,
            selected_fields,
            limit,
        }) => {
            let projection = selected_fields.as_ref().map(|f| &f.fields);
            let updates = builder
                .build_updates(&model, record_filter, args, projection, limit)
                .map_err(TranslateError::QueryBuildFailure)?
                .into_iter()
                .map(if projection.is_some() {
                    Expression::Query
                } else {
                    Expression::Execute
                })
                .collect::<Vec<_>>();

            if let Some(selected_fields) = selected_fields {
                let mut expr = Expression::Concat(updates);
                if !selected_fields.nested.is_empty() {
                    expr = add_inmemory_join(expr, selected_fields.nested, builder)?;
                }
                expr
            } else {
                Expression::Sum(updates)
            }
        }

        WriteQuery::UpdateRecord(UpdateRecord::WithSelection(UpdateRecordWithSelection {
            name: _,
            model,
            record_filter,
            args,
            selected_fields,
            selection_order: _,
        })) => {
            let query = if args.is_empty() {
                // if there's no args we can just issue a read query
                let args = QueryArguments::from((model.clone(), record_filter.filter)).with_take(Take::Some(1));
                builder
                    .build_get_records(&model, args, &selected_fields, RelationLoadStrategy::Query)
                    .map_err(TranslateError::QueryBuildFailure)?
            } else {
                builder
                    .build_update(&model, record_filter, args, Some(&selected_fields))
                    .map_err(TranslateError::QueryBuildFailure)?
            };
            Expression::Unique(Box::new(Expression::Query(query)))
        }

        WriteQuery::Upsert(upsert) => {
            let query = builder
                .build_upsert(
                    upsert.model(),
                    upsert.filter().clone(),
                    upsert.create().clone(),
                    upsert.update().clone(),
                    upsert.selected_fields(),
                    &upsert.unique_constraints(),
                )
                .map_err(TranslateError::QueryBuildFailure)?;
            Expression::Unique(Box::new(Expression::Query(query)))
        }

        WriteQuery::QueryRaw(RawQuery {
            model,
            inputs,
            query_type,
        }) => Expression::Query(
            builder
                .build_raw(model.as_ref(), inputs, query_type)
                .map_err(TranslateError::QueryBuildFailure)?,
        ),

        WriteQuery::ExecuteRaw(RawQuery {
            model,
            inputs,
            query_type,
        }) => Expression::Execute(
            builder
                .build_raw(model.as_ref(), inputs, query_type)
                .map_err(TranslateError::QueryBuildFailure)?,
        ),

        WriteQuery::DeleteRecord(DeleteRecord {
            name: _,
            model,
            record_filter,
            selected_fields,
        }) => {
            let selected_fields = selected_fields.as_ref().map(|sf| &sf.fields);
            let query = builder
                .build_delete(&model, record_filter, selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;
            if selected_fields.is_some() {
                Expression::Unique(Box::new(Expression::Query(query)))
            } else {
                Expression::Execute(query)
            }
        }

        WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model,
            record_filter,
            limit,
        }) => Expression::Sum(
            builder
                .build_deletes(&model, record_filter, limit)
                .map_err(TranslateError::QueryBuildFailure)?
                .into_iter()
                .map(Expression::Execute)
                .collect::<Vec<_>>(),
        ),

        WriteQuery::ConnectRecords(ConnectRecords {
            parent_id,
            child_ids,
            relation_field,
        }) => {
            let (_, parent) = parent_id
                .into_iter()
                .flat_map(IntoIterator::into_iter)
                .exactly_one()
                .expect("query compiler connects should never have more than one parent expression");
            let (_, child) = child_ids
                .into_iter()
                .flat_map(IntoIterator::into_iter)
                .exactly_one()
                .expect("query compiler connects should never have more than one child expression");
            let query = builder
                .build_m2m_connect(relation_field, parent, child)
                .map_err(TranslateError::QueryBuildFailure)?;
            Expression::Execute(query)
        }

        WriteQuery::DisconnectRecords(DisconnectRecords {
            parent_id,
            child_ids,
            relation_field,
        }) => {
            let parent_id = parent_id.as_ref().expect("should have parent ID for disconnect");
            let query = builder
                .build_m2m_disconnect(relation_field, parent_id, &child_ids)
                .map_err(TranslateError::QueryBuildFailure)?;
            Expression::Execute(query)
        }

        other => {
            return Err(TranslateError::QueryBuildFailure(Box::new(UnhandledBranch(other))));
        }
    })
}

#[derive(Debug, Error)]
#[error("unimplemented write query: {0:?}")]
struct UnhandledBranch(WriteQuery);
