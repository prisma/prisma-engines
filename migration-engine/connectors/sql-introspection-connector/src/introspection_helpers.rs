//! Small utility functions.

use sql::walkers::TableWalker;
use sql_schema_describer::{self as sql, ColumnTypeFamily, IndexType};
use std::cmp;

/// This function implements the reverse behaviour of the `Ord` implementation for `Option`: it
/// puts `None` values last, and otherwise orders `Some`s by their contents, like the `Ord` impl.
pub(crate) fn compare_options_none_last<T: cmp::Ord>(a: Option<T>, b: Option<T>) -> cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => cmp::Ordering::Less,
        (None, Some(_)) => cmp::Ordering::Greater,
        (None, None) => cmp::Ordering::Equal,
    }
}

/// True if the table is a legacy migrations table, which we do not
/// want in the PSL.
pub(crate) fn is_old_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_Migration"
        && table.columns().any(|c| c.name() == "revision")
        && table.columns().any(|c| c.name() == "name")
        && table.columns().any(|c| c.name() == "datamodel")
        && table.columns().any(|c| c.name() == "status")
        && table.columns().any(|c| c.name() == "applied")
        && table.columns().any(|c| c.name() == "rolled_back")
        && table.columns().any(|c| c.name() == "datamodel_steps")
        && table.columns().any(|c| c.name() == "database_migration")
        && table.columns().any(|c| c.name() == "errors")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "finished_at")
}

/// True if the table is a migrations table, which we do not want in
/// the PSL.
pub(crate) fn is_new_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_prisma_migrations"
        && table.columns().any(|c| c.name() == "id")
        && table.columns().any(|c| c.name() == "checksum")
        && table.columns().any(|c| c.name() == "finished_at")
        && table.columns().any(|c| c.name() == "migration_name")
        && table.columns().any(|c| c.name() == "logs")
        && table.columns().any(|c| c.name() == "rolled_back_at")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "applied_steps_count")
}

/// A legacy Prisma1 table, not to be rendered in the PSL.
pub(crate) fn is_relay_table(table: TableWalker<'_>) -> bool {
    table.name() == "_RelayId"
        && table.column("id").is_some()
        && table
            .columns()
            .any(|col| col.name().eq_ignore_ascii_case("stablemodelidentifier"))
}

/// If the table has `createdAt` and `updatedAt` fields.
pub(crate) fn has_created_at_and_updated_at(table: TableWalker<'_>) -> bool {
    let has_created_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("createdat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    let has_updated_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("updatedat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    has_created_at && has_updated_at
}

/// A special table to handle many to many relations in Prisma.
pub(crate) fn is_prisma_join_table(t: TableWalker<'_>) -> bool {
    is_prisma_1_point_0_join_table(t) || is_prisma_1_point_1_or_2_join_table(t)
}

/// A legacy Prisma 1 list table.
pub(crate) fn is_prisma_1_or_11_list_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.columns().any(|col| col.name().eq_ignore_ascii_case("nodeid"))
        && table.column("position").is_some()
        && table.column("value").is_some()
}

/// A special legacy join table, seen in Prisma 1.1 and 1.2.
pub(crate) fn is_prisma_1_point_1_or_2_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 2 && table.indexes().len() >= 2 && common_prisma_m_to_n_relation_conditions(table)
}

/// A special legacy join table, seen in Prisma 1.0.
pub(crate) fn is_prisma_1_point_0_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.indexes().len() >= 2
        && table.columns().any(|c| c.name() == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

/// If a relation defines a Prisma many to many relation.
fn common_prisma_m_to_n_relation_conditions(table: TableWalker<'_>) -> bool {
    fn is_a(column: &str) -> bool {
        column.eq_ignore_ascii_case("a")
    }

    fn is_b(column: &str) -> bool {
        column.eq_ignore_ascii_case("b")
    }

    let mut fks = table.foreign_keys();
    let first_fk = fks.next();
    let second_fk = fks.next();
    let a_b_match = || {
        let first_fk = first_fk.unwrap();
        let second_fk = second_fk.unwrap();
        let first_fk_col = first_fk.constrained_columns().next().unwrap().name();
        let second_fk_col = second_fk.constrained_columns().next().unwrap().name();
        (first_fk.referenced_table().name() <= second_fk.referenced_table().name()
            && is_a(first_fk_col)
            && is_b(second_fk_col))
            || (second_fk.referenced_table().name() <= first_fk.referenced_table().name()
                && is_b(first_fk_col)
                && is_a(second_fk_col))
    };
    table.name().starts_with('_')
        //UNIQUE INDEX [A,B]
        && table.indexes().any(|i| {
            i.columns().len() == 2
                && is_a(i.columns().next().unwrap().as_column().name())
                && is_b(i.columns().nth(1).unwrap().as_column().name())
                && i.is_unique()
        })
    //INDEX [B]
    && table
        .indexes()
        .any(|i| i.columns().len() == 1 && is_b(i.columns().next().unwrap().as_column().name()) && i.index_type() == IndexType::Normal)
        // 2 FKs
        && table.foreign_keys().len() == 2
        // Lexicographically lower model referenced by A
        && a_b_match()
}
