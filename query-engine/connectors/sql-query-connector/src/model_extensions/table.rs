use std::sync::Arc;

use crate::{model_extensions::AsColumns, Context};
use prisma_models::Model;
use quaint::{
    ast::{Column, Table},
    prelude::IndexDefinition,
};

pub(crate) fn db_name_with_schema(model: &Model, ctx: &Context<'_>) -> Table<'static> {
    let schema_prefix = model
        .walker()
        .schema_name()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| ctx.schema_name().to_owned());
    let model_db_name = model.db_name().to_owned();
    (schema_prefix, model_db_name).into()
}

pub(crate) trait AsTable {
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static> {
        let mut table: Table<'static> = db_name_with_schema(self, ctx);
        let mut indexes = Vec::new();

        let id_cols: Vec<Column<'static>> = self
            .primary_identifier()
            .as_scalar_fields()
            .expect("Primary identifier has non-scalar fields.")
            .as_columns(ctx)
            .collect();
        indexes.push(IndexDefinition::Compound(id_cols));

        for index in self.unique_indexes() {
            let fields: Vec<_> = index
                .fields()
                .map(|f| prisma_models::ScalarFieldRef::from((self.dm.clone(), f)))
                .collect();
            let index: Vec<Column<'static>> = fields.as_columns(ctx).collect();
            indexes.push(index.into())
        }

        table.set_unique_indexes(Arc::new(indexes));
        table
    }
}
