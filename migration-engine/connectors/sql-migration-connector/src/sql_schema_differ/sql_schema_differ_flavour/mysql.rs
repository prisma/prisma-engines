use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::MysqlFlavour, flavour::MYSQL_IDENTIFIER_SIZE_LIMIT, pair::Pair, sql_schema_differ::column::ColumnDiffer,
    sql_schema_differ::ColumnTypeChange,
};
use sql_schema_describer::{walkers::IndexWalker, ColumnTypeFamily};

/// On MariaDB, JSON is an alias for LONGTEXT. https://mariadb.com/kb/en/json-data-type/
const MARIADB_ALIASES: &[ColumnTypeFamily] = &[ColumnTypeFamily::String, ColumnTypeFamily::Json];

impl SqlSchemaDifferFlavour for MysqlFlavour {
    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        if differ.database_info.is_mariadb()
            && MARIADB_ALIASES.contains(&differ.previous.column_type_family())
            && MARIADB_ALIASES.contains(&differ.next.column_type_family())
        {
            return None;
        }

        if differ.previous.column_type_family() != differ.next.column_type_family() {
            return match (differ.previous.column_type_family(), differ.next.column_type_family()) {
                (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
                (ColumnTypeFamily::String, ColumnTypeFamily::Int) => Some(ColumnTypeChange::RiskyCast),
                (_, _) => Some(ColumnTypeChange::RiskyCast),
            };
        }

        if let (Some(previous_enum), Some(next_enum)) = (
            differ.previous.column_type_family_as_enum(),
            differ.next.column_type_family_as_enum(),
        ) {
            if previous_enum.values == next_enum.values {
                return None;
            }

            return if previous_enum
                .values
                .iter()
                .all(|previous_value| next_enum.values.iter().any(|next_value| previous_value == next_value))
            {
                Some(ColumnTypeChange::SafeCast)
            } else {
                Some(ColumnTypeChange::RiskyCast)
            };
        }

        None
    }

    fn index_should_be_renamed(&self, indexes: &Pair<IndexWalker<'_>>) -> bool {
        // Implements correct comparison for truncated index names.
        let (previous_name, next_name) = indexes.as_ref().map(|idx| idx.name()).into_tuple();

        if previous_name.len() == MYSQL_IDENTIFIER_SIZE_LIMIT && next_name.len() > MYSQL_IDENTIFIER_SIZE_LIMIT {
            previous_name[0..MYSQL_IDENTIFIER_SIZE_LIMIT] != next_name[0..MYSQL_IDENTIFIER_SIZE_LIMIT]
        } else {
            previous_name != next_name
        }
    }

    fn should_create_indexes_from_created_tables(&self) -> bool {
        false
    }
}
