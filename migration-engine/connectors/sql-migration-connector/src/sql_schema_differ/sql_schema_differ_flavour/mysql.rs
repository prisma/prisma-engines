use super::SqlSchemaDifferFlavour;
use crate::{flavour::MysqlFlavour, flavour::MYSQL_IDENTIFIER_SIZE_LIMIT, sql_schema_differ::ColumnDiffer};
use sql_schema_describer::{walkers::IndexWalker, ColumnTypeFamily};

/// On MariaDB, JSON is an alias for LONGTEXT. https://mariadb.com/kb/en/json-data-type/
const MARIADB_ALIASES: &[ColumnTypeFamily] = &[ColumnTypeFamily::String, ColumnTypeFamily::Json];

impl SqlSchemaDifferFlavour for MysqlFlavour {
    fn column_type_changed(&self, differ: &ColumnDiffer<'_>) -> bool {
        if differ.database_info.is_mariadb()
            && MARIADB_ALIASES.contains(&differ.previous.column_type_family())
            && MARIADB_ALIASES.contains(&differ.next.column_type_family())
        {
            return false;
        }

        if differ.previous.column_type_family() != differ.next.column_type_family() {
            return true;
        }

        if let (Some(previous_enum), Some(next_enum)) = (
            differ.previous.column_type_family_as_enum(),
            differ.next.column_type_family_as_enum(),
        ) {
            return previous_enum.values != next_enum.values;
        }

        false
    }

    fn index_should_be_renamed(&self, previous: &IndexWalker<'_>, next: &IndexWalker<'_>) -> bool {
        // Implements correct comparison for truncated index names.
        if previous.name().len() == MYSQL_IDENTIFIER_SIZE_LIMIT && next.name().len() > MYSQL_IDENTIFIER_SIZE_LIMIT {
            previous.name()[0..MYSQL_IDENTIFIER_SIZE_LIMIT] != next.name()[0..MYSQL_IDENTIFIER_SIZE_LIMIT]
        } else {
            previous.name() != next.name()
        }
    }
}
