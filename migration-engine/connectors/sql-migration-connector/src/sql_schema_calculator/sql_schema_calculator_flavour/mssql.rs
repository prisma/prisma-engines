use super::SqlSchemaCalculatorFlavour;
use crate::flavour::{MssqlFlavour, SqlFlavour};
use datamodel::{datamodel_connector::walker_ext_traits::DefaultValueExt, parser_database::walkers::*};
use sql_schema_describer::{mssql::MssqlSchemaExt, ForeignKeyAction};

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn default_constraint_name(&self, default_value: DefaultValueWalker<'_>) -> Option<String> {
        Some(default_value.constraint_name(self.datamodel_connector()).into_owned())
    }

    fn m2m_foreign_key_action(&self, model_a: ModelWalker<'_>, model_b: ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }

    fn push_connector_data(&self, context: &mut super::super::Context<'_>) {
        let mut data = MssqlSchemaExt::default();

        for (table_idx, model) in context.datamodel.db.walk_models().enumerate() {
            let table_id = sql_schema_describer::TableId(table_idx as u32);
            if model.primary_key().and_then(|pk| pk.clustered()) == Some(false) {
                data.nonclustered_primary_keys.push(table_id);
            }

            for (index_index, index) in model.indexes().enumerate() {
                if index.clustered() == Some(true) {
                    data.clustered_indexes
                        .push(sql_schema_describer::IndexId(table_id, index_index as u32))
                }
            }
        }

        context.schema.describer_schema.set_connector_data(Box::new(data));
    }
}
