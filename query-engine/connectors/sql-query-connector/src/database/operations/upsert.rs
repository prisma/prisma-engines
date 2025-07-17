use crate::{Queryable, row::ToSqlRow};
use connector_interface::NativeUpsert;
use query_structure::{ModelProjection, Record, SingleRecord};
use sql_query_builder::{Context, column_metadata, write};

pub(crate) async fn native_upsert(
    conn: &dyn Queryable,
    upsert: NativeUpsert,
    ctx: &Context<'_>,
) -> crate::Result<SingleRecord> {
    let selected_fields: ModelProjection = upsert.selected_fields().into();
    let field_names: Vec<_> = selected_fields.db_names().collect();
    let idents = selected_fields.type_identifiers_with_arities();
    let meta = column_metadata::create(&field_names, &idents);

    let query = write::native_upsert(
        upsert.model(),
        upsert.filter().clone(),
        upsert.create().clone(),
        upsert.update().clone(),
        &upsert.selected_fields().into(),
        &upsert.unique_constraints(),
        ctx,
    );

    let result_set = conn.query(query.into()).await?;

    let row = result_set.into_single()?;
    let record = Record::from(row.to_sql_row(&meta)?);

    Ok(SingleRecord { record, field_names })
}
