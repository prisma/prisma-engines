use crate::sql_schema_differ::{ColumnChange, ColumnChanges, ColumnDiffer, DiffingOptions};
use quaint::prelude::SqlFamily;
use sql_schema_describer::{Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue};

pub(crate) fn expand_alter_column(
    previous_column: &Column,
    next_column: &Column,
    sql_family: &SqlFamily,
    diffing_options: &DiffingOptions,
) -> Option<ExpandedAlterColumn> {
    let column_differ = ColumnDiffer {
        diffing_options,
        previous: previous_column,
        next: next_column,
    };

    match sql_family {
        SqlFamily::Sqlite => expand_sqlite_alter_column(&column_differ).map(ExpandedAlterColumn::Sqlite),
        SqlFamily::Mysql => expand_mysql_alter_column(&column_differ).map(ExpandedAlterColumn::Mysql),
        SqlFamily::Postgres => expand_postgres_alter_column(&column_differ).map(ExpandedAlterColumn::Postgres),
    }
}

pub(crate) fn expand_sqlite_alter_column(_columns: &ColumnDiffer) -> Option<Vec<SqliteAlterColumn>> {
    None
}

pub(crate) fn expand_mysql_alter_column(columns: &ColumnDiffer) -> Option<Vec<MysqlAlterColumn>> {
    let mut changes: Vec<MysqlAlterColumn> = Vec::new();

    // Whether we should issue a MODIFY statement to redefine the column.
    let mut modify = false;

    for change in columns.all_changes().iter() {
        match change {
            ColumnChange::Default => match (&columns.previous.default, &columns.next.default) {
                (_, Some(_)) => modify = true,
                (_, None) => changes.push(MysqlAlterColumn::DropDefault),
            },
            ColumnChange::Arity | ColumnChange::Type => modify = true,
            ColumnChange::Renaming => unreachable!("Column renaming on MySQL"),
            _ => return None,
        }
    }

    if modify {
        let alter_column = MysqlAlterColumn::Modify {
            column_type: ColumnType {
                raw: if !columns.all_changes().type_changed()
                    && columns
                        .next
                        .default
                        .as_ref()
                        .map(|default| !matches!(default, DefaultValue::NOW))
                        .unwrap_or(true)
                {
                    columns.previous.tpe.raw.clone()
                } else {
                    "".into()
                },
                family: columns.next.tpe.family.clone(),
                arity: columns.next.tpe.arity.clone(),
            },
            default: columns.next.default.clone(),
            changes: columns.all_changes(),
        };

        return Some(vec![alter_column]);
    }

    Some(changes)
}

pub(crate) fn expand_postgres_alter_column(columns: &ColumnDiffer) -> Option<Vec<PostgresAlterColumn>> {
    let mut changes = Vec::new();

    for change in columns.all_changes().iter() {
        match change {
            ColumnChange::Default => match (&columns.previous.default, &columns.next.default) {
                (_, Some(next_default)) => changes.push(PostgresAlterColumn::SetDefault(next_default.clone())),
                (_, None) => changes.push(PostgresAlterColumn::DropDefault),
            },
            ColumnChange::Arity => match (&columns.previous.tpe.arity, &columns.next.tpe.arity) {
                (ColumnArity::Required, ColumnArity::Nullable) => changes.push(PostgresAlterColumn::DropNotNull),
                _ => return None,
            },
            ColumnChange::Type => match (&columns.previous.tpe.family, &columns.next.tpe.family) {
                // Ints can be cast to text.
                (ColumnTypeFamily::Int, ColumnTypeFamily::String) => {
                    changes.push(PostgresAlterColumn::SetType(columns.next.tpe.clone()))
                }
                _ => return None,
            },
            ColumnChange::Renaming => unreachable!("column renaming"),
        }
    }

    Some(changes)
}

#[derive(Debug)]
pub(crate) enum ExpandedAlterColumn {
    Postgres(Vec<PostgresAlterColumn>),
    Mysql(Vec<MysqlAlterColumn>),
    Sqlite(Vec<SqliteAlterColumn>),
}

#[derive(Debug)]
/// https://www.postgresql.org/docs/9.1/sql-altertable.html
pub(crate) enum PostgresAlterColumn {
    SetDefault(sql_schema_describer::DefaultValue),
    DropDefault,
    DropNotNull,
    SetType(ColumnType),
    // Not used yet:
    // SetNotNull,
    // Rename { previous_name: String, next_name: String },
}

/// https://dev.mysql.com/doc/refman/8.0/en/alter-table.html
#[derive(Debug)]
pub(crate) enum MysqlAlterColumn {
    // SetDefault(sql_schema_describer::DefaultValue),
    DropDefault,
    Modify {
        column_type: ColumnType,
        default: Option<DefaultValue>,
        changes: ColumnChanges,
    },
}

#[derive(Debug)]
pub(crate) enum SqliteAlterColumn {
    // Not used yet:
// Rename { previous_name: String, next_name: String },
}
