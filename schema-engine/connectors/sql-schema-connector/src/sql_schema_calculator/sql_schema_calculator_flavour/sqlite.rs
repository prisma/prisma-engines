use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;
use psl::parser_database::{walkers::*, ScalarType};

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: ScalarFieldWalker<'_>) -> bool {
        field
            .model()
            .primary_key()
            .map(|pk| pk.contains_exactly_fields([field].into_iter()))
            .unwrap_or(false)
            && field.scalar_type() == Some(ScalarType::Int)
    }
}
