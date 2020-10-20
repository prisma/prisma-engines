use crate::sql_schema_differ::{ColumnChange, ColumnChanges, ColumnDiffer};
use sql_schema_describer::{ColumnArity, ColumnType, DefaultValue};

pub(crate) fn expand_mysql_alter_column(columns: &ColumnDiffer<'_>, changes: &ColumnChanges) -> MysqlAlterColumn {
    if changes.only_default_changed() && columns.next.default().is_none() {
        return MysqlAlterColumn::DropDefault;
    }

    if changes.column_was_renamed() {
        unreachable!("MySQL column renaming.")
    }

    // @default(dbgenerated()) does not give us the information in the prisma schema, so we have to
    // transfer it from the introspected current state of the database.
    let new_default = match (&columns.previous.default(), &columns.next.default()) {
        (Some(DefaultValue::DBGENERATED(previous)), Some(DefaultValue::DBGENERATED(next)))
            if next.is_empty() && !previous.is_empty() =>
        {
            Some(DefaultValue::DBGENERATED(previous.clone()))
        }
        _ => columns.next.default().cloned(),
    };

    MysqlAlterColumn::Modify {
        changes: changes.clone(),
        new_default,
    }
}

pub(crate) fn expand_postgres_alter_column(
    columns: &ColumnDiffer<'_>,
    column_changes: &ColumnChanges,
) -> Vec<PostgresAlterColumn> {
    let mut changes = Vec::new();
    let mut set_type = false;

    for change in column_changes.iter() {
        match change {
            ColumnChange::Default => match (&columns.previous.default(), &columns.next.default()) {
                (_, Some(next_default)) => changes.push(PostgresAlterColumn::SetDefault((**next_default).clone())),
                (_, None) => changes.push(PostgresAlterColumn::DropDefault),
            },
            ColumnChange::Arity => match (&columns.previous.arity(), &columns.next.arity()) {
                (ColumnArity::Required, ColumnArity::Nullable) => changes.push(PostgresAlterColumn::DropNotNull),
                (ColumnArity::Nullable, ColumnArity::Required) => changes.push(PostgresAlterColumn::SetNotNull),
                (ColumnArity::List, ColumnArity::Nullable) => {
                    set_type = true;
                    changes.push(PostgresAlterColumn::DropNotNull)
                }
                (ColumnArity::List, ColumnArity::Required) => {
                    set_type = true;
                    changes.push(PostgresAlterColumn::SetNotNull)
                }
                (ColumnArity::Nullable, ColumnArity::List) | (ColumnArity::Required, ColumnArity::List) => {
                    set_type = true;
                }
                (ColumnArity::Nullable, ColumnArity::Nullable)
                | (ColumnArity::Required, ColumnArity::Required)
                | (ColumnArity::List, ColumnArity::List) => (),
            },
            ColumnChange::TypeChanged => set_type = true,
            ColumnChange::Sequence => {
                if columns.previous.is_autoincrement() {
                    // The sequence should be dropped.
                    changes.push(PostgresAlterColumn::DropDefault)
                } else {
                    // The sequence should be created.
                    changes.push(PostgresAlterColumn::AddSequence)
                }
            }
            ColumnChange::Renaming => unreachable!("column renaming"),
        }
    }

    // This is a flag so we don't push multiple SetTypes from arity and type changes.
    if set_type {
        changes.push(PostgresAlterColumn::SetType(columns.next.column_type().clone()));
    }

    changes
}

#[derive(Debug)]
/// https://www.postgresql.org/docs/9.1/sql-altertable.html
pub(crate) enum PostgresAlterColumn {
    SetDefault(sql_schema_describer::DefaultValue),
    DropDefault,
    DropNotNull,
    SetType(ColumnType),
    SetNotNull,
    /// Add an auto-incrementing sequence as a default on the column.
    AddSequence,
}

/// https://dev.mysql.com/doc/refman/8.0/en/alter-table.html
///
/// We don't use SET DEFAULT because it can't be used to set the default to an expression on most
/// MySQL versions. We use MODIFY for default changes instead.
#[derive(Debug)]
pub(crate) enum MysqlAlterColumn {
    DropDefault,
    Modify {
        new_default: Option<DefaultValue>,
        changes: ColumnChanges,
    },
}

// Not used yet: SQLite only supports column renamings, which we don't. All
// other transformations will involve redefining the table.
// https://www.sqlite.org/lang_altertable.html
#[derive(Debug)]
pub(crate) enum SqliteAlterColumn {}
