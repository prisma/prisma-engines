use query_structure::ModelProjection;
use sql_query_connector::{context::Context, generate_insert_statements, query_builder};

use crate::{
    compiler::{expression::Expression, translate::TranslateResult},
    WriteQuery,
};

use super::build_db_query;

pub(crate) fn translate_write_query(query: WriteQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    Ok(match query {
        WriteQuery::CreateRecord(cr) => {
            // TODO: MySQL needs additional logic to generate IDs on our side.
            // See sql_query_connector::database::operations::write::create_record
            let query = query_builder::write::create_record(
                &cr.model,
                cr.args,
                &ModelProjection::from(&cr.selected_fields),
                ctx,
            );

            // TODO: we probably need some additional node type or extra info in the WriteQuery node
            // to help the client executor figure out the returned ID in the case when it's inferred
            // from the query arguments.
            Expression::Query(build_db_query(query)?)
        }

        WriteQuery::CreateManyRecords(cmr) => {
            if let Some(selected_fields) = cmr.selected_fields {
                Expression::Concat(
                    generate_insert_statements(
                        &cmr.model,
                        cmr.args,
                        cmr.skip_duplicates,
                        Some(&selected_fields.fields.into()),
                        ctx,
                    )
                    .into_iter()
                    .map(build_db_query)
                    .map(|maybe_db_query| maybe_db_query.map(Expression::Execute))
                    .collect::<TranslateResult<Vec<_>>>()?,
                )
            } else {
                Expression::Sum(
                    generate_insert_statements(&cmr.model, cmr.args, cmr.skip_duplicates, None, ctx)
                        .into_iter()
                        .map(build_db_query)
                        .map(|maybe_db_query| maybe_db_query.map(Expression::Execute))
                        .collect::<TranslateResult<Vec<_>>>()?,
                )
            }
        }

        _ => todo!(),
    })
}
