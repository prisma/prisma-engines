use crate::sql_schema_differ::{ColumnChange, ColumnChanges, ColumnDiffer};
use quaint::prelude::SqlFamily;
use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue};

pub(crate) fn expand_alter_column(
    column_differ: &ColumnDiffer<'_>,
    sql_family: &SqlFamily,
) -> Option<ExpandedAlterColumn> {
    match sql_family {
        SqlFamily::Sqlite => expand_sqlite_alter_column(&column_differ).map(ExpandedAlterColumn::Sqlite),
        SqlFamily::Mysql => Some(ExpandedAlterColumn::Mysql(expand_mysql_alter_column(&column_differ))),
        SqlFamily::Postgres => expand_postgres_alter_column(&column_differ).map(ExpandedAlterColumn::Postgres),
        SqlFamily::Mssql => todo!("Greetings from Redmond"),
    }
}

pub(crate) fn expand_sqlite_alter_column(_columns: &ColumnDiffer<'_>) -> Option<Vec<SqliteAlterColumn>> {
    None
}

pub(crate) fn expand_mysql_alter_column(columns: &ColumnDiffer<'_>) -> MysqlAlterColumn {
    let column_changes = columns.all_changes();

    if column_changes.only_default_changed() && columns.next.default().is_none() {
        return MysqlAlterColumn::DropDefault;
    }

    if column_changes.column_was_renamed() {
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
        changes: column_changes,
        new_default,
    }
}

pub(crate) fn expand_postgres_alter_column(columns: &ColumnDiffer<'_>) -> Option<Vec<PostgresAlterColumn>> {
    let mut changes = Vec::new();

    for change in columns.all_changes().iter() {
        match change {
            ColumnChange::Default => match (&columns.previous.default(), &columns.next.default()) {
                (_, Some(next_default)) => changes.push(PostgresAlterColumn::SetDefault((**next_default).clone())),
                (_, None) => changes.push(PostgresAlterColumn::DropDefault),
            },
            ColumnChange::Arity => match (&columns.previous.arity(), &columns.next.arity()) {
                (ColumnArity::Required, ColumnArity::Nullable) => changes.push(PostgresAlterColumn::DropNotNull),
                (ColumnArity::Nullable, ColumnArity::Required) => changes.push(PostgresAlterColumn::SetNotNull),
                (ColumnArity::List, ColumnArity::Nullable) => {
                    changes.push(PostgresAlterColumn::SetType(columns.next.column_type().clone()));
                    changes.push(PostgresAlterColumn::DropNotNull)
                }
                (ColumnArity::List, ColumnArity::Required) => {
                    changes.push(PostgresAlterColumn::SetType(columns.next.column_type().clone()));
                    changes.push(PostgresAlterColumn::SetNotNull)
                }
                (ColumnArity::Nullable, ColumnArity::List) | (ColumnArity::Required, ColumnArity::List) => {
                    changes.push(PostgresAlterColumn::SetType(columns.next.column_type().clone()))
                }
                (ColumnArity::Nullable, ColumnArity::Nullable)
                | (ColumnArity::Required, ColumnArity::Required)
                | (ColumnArity::List, ColumnArity::List) => (),
            },
            ColumnChange::Type => match (
                &columns.previous.column_type_family(),
                &columns.next.column_type_family(),
            ) {
                // Ints can be cast to text.
                (ColumnTypeFamily::Int, ColumnTypeFamily::String) => {
                    changes.push(PostgresAlterColumn::SetType(columns.next.column_type().clone()))
                }
                _ => return None,
            },
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

    Some(changes)
}

#[derive(Debug)]
pub(crate) enum ExpandedAlterColumn {
    Postgres(Vec<PostgresAlterColumn>),
    Mysql(MysqlAlterColumn),
    Sqlite(Vec<SqliteAlterColumn>),
}

#[derive(Debug)]
/// https://www.postgresql.org/docs/9.1/sql-altertable.html
pub(crate) enum PostgresAlterColumn {
    SetDefault(sql_schema_describer::DefaultValue),
    DropDefault,
    DropNotNull,
    SetType(ColumnType),
    SetNotNull,
    // Add an auto-incrementing sequence as a default on the column.
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

#[derive(Debug)]
pub(crate) enum SqliteAlterColumn {
    // Not used yet
}
