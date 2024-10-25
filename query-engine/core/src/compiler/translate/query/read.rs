use query_structure::ModelProjection;
use sql_query_connector::{
    context::Context, model_extensions::AsColumns, query_arguments_ext::QueryArgumentsExt, query_builder,
};

use crate::{
    compiler::{expression::Expression, translate::TranslateResult},
    ReadQuery, RelatedRecordsQuery,
};

use super::build_db_query;

pub(crate) fn translate_read_query(query: ReadQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = rq.selected_fields.without_relations().into_virtuals_last();

            let query = query_builder::read::get_records(
                &rq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                rq.filter.expect("ReadOne query should always have filter set"),
                ctx,
            )
            .limit(1);

            Expression::Query(build_db_query(query)?)
        }

        ReadQuery::ManyRecordsQuery(mrq) => {
            let selected_fields = mrq.selected_fields.without_relations().into_virtuals_last();
            let needs_reversed_order = mrq.args.needs_reversed_order();

            // TODO: we ignore chunking for now
            let query = query_builder::read::get_records(
                &mrq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                mrq.args,
                ctx,
            );

            let expr = Expression::Query(build_db_query(query)?);

            if needs_reversed_order {
                Expression::Reverse(Box::new(expr))
            } else {
                expr
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            if rrq.parent_field.relation().is_many_to_many() {
                build_read_m2m_query(rrq, ctx)?
            } else {
                build_read_one2m_query(rrq, ctx)?
            }
        }

        _ => unimplemented!(),
    })
}

fn build_read_m2m_query(query: RelatedRecordsQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    todo!()
}

fn build_read_one2m_query(query: RelatedRecordsQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    todo!()
}
