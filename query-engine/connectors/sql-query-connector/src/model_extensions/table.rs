use crate::{model_extensions::AsColumns, Context};
use prisma_models::Model;
use quaint::ast::{Column, Table};

pub(crate) fn db_name_with_schema(model: &Model, ctx: &Context<'_>) -> Table<'static> {
    let schema_prefix = model.schema_name().unwrap_or_else(|| ctx.schema_name().to_owned());
    let model_db_name = model.db_name().to_string();
    (schema_prefix, model_db_name).into()
}

pub(crate) trait AsTable {
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static> {
        let table: Table<'static> = db_name_with_schema(self, ctx);

        let id_cols: Vec<Column<'static>> = self
            .primary_identifier()
            .as_scalar_fields()
            .expect("Primary identifier has non-scalar fields.")
            .as_columns(ctx)
            .collect();

        let table = table.add_unique_index(id_cols);

        self.unique_indexes().into_iter().fold(table, |table, index| {
            let fields: Vec<_> = index
                .fields()
                .map(|f| prisma_models::ScalarFieldRef::from((self.dm.clone(), f)))
                .collect();
            let index: Vec<Column<'static>> = fields.as_columns(ctx).collect();
            table.add_unique_index(index)
        })
    }
}
