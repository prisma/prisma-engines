use super::SqlSchemaDifferFlavour;
use crate::flavour::MssqlFlavour;
use sql_schema_describer::walkers::IndexWalker;

impl SqlSchemaDifferFlavour for MssqlFlavour {
    fn should_skip_index_for_new_table(&self, index: &IndexWalker<'_>) -> bool {
        index.index_type().is_unique()
    }
}
