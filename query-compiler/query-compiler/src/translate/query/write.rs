use query_builder::QueryBuilder;
use query_core::WriteQuery;

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
                        .map(Expression::Execute)
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

        other => todo!("{other:?}"),
    })
}
