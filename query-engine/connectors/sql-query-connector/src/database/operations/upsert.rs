use crate::{
    column_metadata,
    filter::FilterBuilder,
    model_extensions::AsColumns,
    query_builder::write::{build_update_and_set_query, create_record},
    row::ToSqlRow,
    Context, Queryable,
};
use connector_interface::NativeUpsert;
use quaint::prelude::{OnConflict, Query};
use query_structure::{ModelProjection, Record, SingleRecord};

pub(crate) async fn native_upsert(
    conn: &dyn Queryable,
    upsert: NativeUpsert,
    ctx: &Context<'_>,
) -> crate::Result<SingleRecord> {
    let selected_fields: ModelProjection = upsert.selected_fields().into();
    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();

    let meta = column_metadata::create(&field_names, &idents);

    let where_condition = FilterBuilder::without_top_level_joins().visit_filter(upsert.filter().clone(), ctx);
    let update =
        build_update_and_set_query(upsert.model(), upsert.update().clone(), None, ctx).so_that(where_condition);

    let insert = create_record(upsert.model(), upsert.create().clone(), &selected_fields, ctx);

    let constraints: Vec<_> = upsert.unique_constraints().as_columns(ctx).collect();
    let query: Query = insert.on_conflict(OnConflict::Update(update, constraints)).into();

    let result_set = conn.query(query).await?;

    let row = result_set.into_single()?;
    let record = Record::from(row.to_sql_row(&meta)?);

    Ok(SingleRecord { record, field_names })
}
