use super::SqlSchemaCalculatorFlavour;
use crate::flavour::{MssqlFlavour, SqlFlavour};
use psl::{
    datamodel_connector::walker_ext_traits::{DefaultValueExt, IndexWalkerExt},
    parser_database::walkers::*,
};
use sql_schema_describer::{
    mssql::{IndexBits, MssqlSchemaExt},
    ForeignKeyAction,
};

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

    fn push_connector_data(&self, context: &mut crate::sql_schema_calculator::Context<'_>) {
        let mut data = MssqlSchemaExt::default();

        for model in context.datamodel.db.walk_models() {
            let table_id = context.model_id_to_table_id[&model.model_id()];
            let table = context.schema.walk(table_id);
            if model
                .primary_key()
                .map(|pk| pk.clustered().is_none() || pk.clustered() == Some(true))
                .unwrap_or(false)
            {
                *data.index_bits.entry(table.primary_key().unwrap().id).or_default() |= IndexBits::Clustered;
            }

            for index in model.indexes() {
                let sql_index = table
                    .indexes()
                    .find(|idx| idx.name() == index.constraint_name(self.datamodel_connector()))
                    .unwrap();

                if index.clustered() == Some(true) {
                    *data.index_bits.entry(sql_index.id).or_default() |= IndexBits::Clustered;
                }
            }
        }

        context.schema.describer_schema.set_connector_data(Box::new(data));
    }
}
