use query_builder::QueryBuilder;
use query_core::{
    ConnectRecords, DeleteManyRecords, DeleteRecord, DisconnectRecords, RawQuery, UpdateManyRecords, UpdateRecord,
    UpdateRecordWithSelection, WriteQuery,
};
use query_structure::QueryArguments;

use crate::{expression::Expression, translate::TranslateResult, TranslateError};

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
            if let Some(selected_fields) = cmr.selected_fields {
                Expression::Concat(
                    builder
                        .build_inserts(&cmr.model, cmr.args, cmr.skip_duplicates, Some(&selected_fields.fields))
                        .map_err(TranslateError::QueryBuildFailure)?
                        .into_iter()
                        .map(Expression::Query)
                        .collect::<Vec<_>>(),
                )
            } else {
                Expression::Sum(
                    builder
                        .build_inserts(&cmr.model, cmr.args, cmr.skip_duplicates, None)
                        .map_err(TranslateError::QueryBuildFailure)?
                        .into_iter()
                        .map(Expression::Execute)
                        .collect::<Vec<_>>(),
                )
            }
        }

        WriteQuery::UpdateManyRecords(UpdateManyRecords {
            model,
            record_filter,
            args,
            selected_fields,
            limit,
            ..
        }) => {
            let projection = selected_fields.as_ref().map(|f| &f.fields);
            let updates = if record_filter.has_selectors() {
                // we'll need to implement the equivalent of:
                // ```
                // let filter = record_filter.filter.clone();
                // let ids = conn.filter_selectors(model, record_filter, ctx).await?;
                // let slice = &ids[..limit.unwrap_or(ids.len()).min(ids.len())];
                // ```
                //
                // and pass it to a builder methods that takes the selectors
                todo!()
            } else {
                builder
                    .build_updates_from_filter(&model, record_filter.filter, args, projection, limit)
                    .map_err(TranslateError::QueryBuildFailure)?
                    .into_iter()
                    .map(if projection.is_some() {
                        Expression::Query
                    } else {
                        Expression::Execute
                    })
                    .collect::<Vec<_>>()
            };
            if projection.is_some() {
                Expression::Concat(updates)
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
            // TODO: we're ignoring selection order
            selection_order: _,
        })) => {
            let query = if args.is_empty() {
                // if there's no args we can just issue a read query
                let args = QueryArguments::from((model.clone(), record_filter.filter)).with_take(Some(1));
                builder
                    .build_get_records(&model, args, &selected_fields)
                    .map_err(TranslateError::QueryBuildFailure)?
            } else {
                builder
                    .build_update(&model, record_filter, args, Some(&selected_fields))
                    .map_err(TranslateError::QueryBuildFailure)?
            };
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
            let parent_id = parent_id.as_ref().expect("should have parent ID for connect");
            let query = builder
                .build_m2m_connect(relation_field, parent_id, &child_ids)
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

        other => todo!("{other:?}"),
    })
}
