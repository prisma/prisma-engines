mod mysql;
mod postgres;
mod sqlite;

/// Trait to specialize SQL schema diffing (resulting in migration steps) by SQL backend.
pub(crate) trait SqlSchemaDifferFlavour {
    /// By implementing this method, the flavour signals the differ that specific tables should be ignored. This is mostly for system tables.
    fn table_should_be_ignored(&self, _table_name: &str) -> bool {
        false
    }
}
