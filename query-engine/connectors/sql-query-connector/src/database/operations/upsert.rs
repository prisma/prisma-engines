use connector_interface::NativeUpsert;
use prisma_models::{ModelProjection, Record, SingleRecord};
use quaint::prelude::{OnConflict, Query};

use crate::{
    column_metadata,
    filter_conversion::AliasedCondition,
    model_extensions::AsColumns,
    query_builder::{build_update_and_set_query, create_record},
    query_ext::QueryExt,
    row::ToSqlRow,
};

pub async fn native_upsert(
    conn: &dyn QueryExt,
    upsert: NativeUpsert,
    trace_id: Option<String>,
) -> crate::Result<SingleRecord> {
    let selected_fields: ModelProjection = upsert.selected_fields().into();
    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();

    let meta = column_metadata::create(&field_names, &idents);

    let where_condition = upsert.filter().aliased_condition_from(None, false);
    let update = build_update_and_set_query(upsert.model(), upsert.update().clone(), None).so_that(where_condition);

    let insert = create_record(&upsert.model(), upsert.create().clone(), trace_id);

    let constraints: Vec<_> = upsert.unique_constraints().as_columns().collect();
    let query: Query = insert
        .on_conflict(OnConflict::Update(update, constraints))
        .returning(selected_fields.as_columns())
        .into();

    let result_set = conn.query(query).await?;

    let row = result_set.into_single()?;
    let record = Record::from(row.to_sql_row(&meta)?);
    Ok(SingleRecord { record, field_names })
}
