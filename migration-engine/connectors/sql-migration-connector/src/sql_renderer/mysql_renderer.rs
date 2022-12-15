use super::{common::*, IteratorJoin, SqlRenderer};
use crate::{
    flavour::MysqlFlavour,
    pair::Pair,
    sql_migration::{AlterColumn, AlterEnum, AlterTable, RedefineTable, TableChange},
    sql_schema_differ::ColumnChanges,
};
use once_cell::sync::Lazy;
use psl::builtin_connectors::MySqlType;
use psl::dml::PrismaValue;
use regex::Regex;
use sql_ddl::{mysql as ddl, IndexColumn, SortOrder};
use sql_schema_describer::{
    walkers::{
        ColumnWalker, EnumWalker, ForeignKeyWalker, IndexWalker, TableWalker, UserDefinedTypeWalker, ViewWalker,
    },
    ColumnTypeFamily, DefaultKind, DefaultValue, ForeignKeyAction, SQLSortOrder, SqlSchema,
};
use std::{borrow::Cow, fmt::Write as _};

impl MysqlFlavour {
    fn render_column<'a>(&self, col: ColumnWalker<'a>) -> ddl::Column<'a> {
        let default = col
            .default()
            .filter(|default| {
                !matches!(default.kind(),  DefaultKind::Sequence(_) | DefaultKind::DbGenerated(None))
                    // We do not want to render JSON defaults because
                    // they are not supported by MySQL.
                    && !matches!(col.column_type_family(), ColumnTypeFamily::Json)
                    // We do not want to render binary defaults because
                    // they are not supported by MySQL.
                    && !matches!(col.column_type_family(), ColumnTypeFamily::Binary if !default.is_db_generated())
            })
            .map(|default| render_default(col, default.inner()));

        ddl::Column {
            column_name: col.name().into(),
            not_null: col.arity().is_required(),
            column_type: render_column_type(col),
            default,
            auto_increment: col.is_autoincrement(),
            ..Default::default()
        }
    }
}

impl SqlRenderer for MysqlFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Backticks(name)
    }

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        ddl::AlterTable {
            table_name: foreign_key.table().name().into(),
            changes: vec![ddl::AlterTableClause::AddForeignKey(ddl::ForeignKey {
                constraint_name: foreign_key.constraint_name().map(From::from),
                constrained_columns: foreign_key
                    .constrained_columns()
                    .map(|c| Cow::Borrowed(c.name()))
                    .collect(),
                referenced_table: foreign_key.referenced_table().name().into(),
                referenced_columns: foreign_key
                    .referenced_columns()
                    .map(|c| Cow::Borrowed(c.name()))
                    .collect(),
                on_delete: Some(match foreign_key.on_delete_action() {
                    ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
                    ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
                    ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
                    ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
                }),
                on_update: Some(match foreign_key.on_update_action() {
                    ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
                    ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
                    ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
                    ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
                }),
            })],
        }
        .to_string()
    }

    fn render_alter_enum(&self, _alter_enum: &AlterEnum, _differ: Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_alter_enum on MySQL")
    }

    fn render_rename_index(&self, indexes: Pair<IndexWalker<'_>>) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_display(&ddl::AlterTable {
                    table_name: indexes.previous.table().name().into(),
                    changes: vec![sql_ddl::mysql::AlterTableClause::RenameIndex {
                        previous_name: indexes.previous.name().into(),
                        next_name: indexes.next.name().into(),
                    }],
                })
            })
        })
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable {
            table_ids: table_index,
            changes,
        } = alter_table;

        let tables = schemas.walk(*table_index);

        let mut lines = Vec::new();

        for change in changes {
            match change {
                TableChange::DropPrimaryKey => lines.push(sql_ddl::mysql::AlterTableClause::DropPrimaryKey.to_string()),
                TableChange::RenamePrimaryKey => unreachable!("No Renaming Primary Keys on Mysql"),
                TableChange::AddPrimaryKey => lines.push(format!(
                    "ADD PRIMARY KEY ({})",
                    tables
                        .next
                        .primary_key_columns()
                        .unwrap()
                        .map(|c| {
                            let mut rendered = format!("{}", self.quote(c.as_column().name()));

                            if let Some(length) = c.length() {
                                write!(rendered, "({})", length).unwrap();
                            }

                            if let Some(sort_order) = c.sort_order() {
                                rendered.push(' ');
                                rendered.push_str(sort_order.as_ref());
                            }

                            rendered
                        })
                        .join(", ")
                )),
                TableChange::AddColumn {
                    column_id,
                    has_virtual_default: _,
                } => {
                    let column = tables.next.walk(*column_id);
                    let col_sql = self.render_column(column);

                    lines.push(format!("ADD COLUMN {}", col_sql));
                }
                TableChange::DropColumn { column_id } => lines.push(
                    sql_ddl::mysql::AlterTableClause::DropColumn {
                        column_name: tables.previous.walk(*column_id).name().into(),
                    }
                    .to_string(),
                ),
                TableChange::AlterColumn(AlterColumn {
                    changes,
                    column_id,
                    type_change: _,
                }) => {
                    let columns = schemas.walk(*column_id);
                    let expanded = MysqlAlterColumn::new(columns, *changes);

                    match expanded {
                        MysqlAlterColumn::DropDefault => lines.push(format!(
                            "ALTER COLUMN {column} DROP DEFAULT",
                            column = Quoted::mysql_ident(columns.previous.name())
                        )),
                        MysqlAlterColumn::Modify { new_default, changes } => {
                            lines.push(render_mysql_modify(&changes, new_default.as_ref(), columns.next))
                        }
                    };
                }
                TableChange::DropAndRecreateColumn { column_id, changes: _ } => {
                    let columns = schemas.walk(*column_id);
                    lines.push(format!("DROP COLUMN `{}`", columns.previous.name()));
                    lines.push(format!("ADD COLUMN {}", self.render_column(columns.next)));
                }
            };
        }

        if lines.is_empty() {
            return Vec::new();
        }

        vec![format!(
            "ALTER TABLE {} {}",
            self.quote(tables.previous.name()),
            lines.join(",\n    ")
        )]
    }

    fn render_create_enum(&self, _create_enum: EnumWalker<'_>) -> Vec<String> {
        unreachable!(
            "Unreachable render_create_enum() on MySQL. enums are defined on each column that uses them on MySQL"
        )
    }

    fn render_create_index(&self, index: IndexWalker<'_>) -> String {
        ddl::CreateIndex {
            r#type: match index.index_type() {
                sql_schema_describer::IndexType::Unique => ddl::IndexType::Unique,
                sql_schema_describer::IndexType::Normal => ddl::IndexType::Normal,
                sql_schema_describer::IndexType::Fulltext => ddl::IndexType::Fulltext,
                sql_schema_describer::IndexType::PrimaryKey => unreachable!(),
            },
            index_name: index.name().into(),
            on: (
                index.table().name().into(),
                index
                    .columns()
                    .map(|c| IndexColumn {
                        name: c.as_column().name().into(),
                        length: c.length(),
                        sort_order: c.sort_order().map(|so| match so {
                            SQLSortOrder::Asc => SortOrder::Asc,
                            SQLSortOrder::Desc => SortOrder::Desc,
                        }),
                        operator_class: None,
                    })
                    .collect(),
            ),
        }
        .to_string()
    }

    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        ddl::CreateTable {
            table_name: &table_name,
            columns: table.columns().map(|col| self.render_column(col)).collect(),
            indexes: table
                .indexes()
                .filter(|idx| !idx.is_primary_key())
                .map(move |index| ddl::IndexClause {
                    index_name: Some(Cow::from(index.name())),
                    r#type: match index.index_type() {
                        sql_schema_describer::IndexType::Unique => ddl::IndexType::Unique,
                        sql_schema_describer::IndexType::Normal => ddl::IndexType::Normal,
                        sql_schema_describer::IndexType::Fulltext => ddl::IndexType::Fulltext,
                        sql_schema_describer::IndexType::PrimaryKey => unreachable!(),
                    },
                    columns: index
                        .columns()
                        .map(|c| IndexColumn {
                            name: c.as_column().name().into(),
                            length: c.length(),
                            sort_order: c.sort_order().map(|so| match so {
                                SQLSortOrder::Asc => SortOrder::Asc,
                                SQLSortOrder::Desc => SortOrder::Desc,
                            }),
                            operator_class: None,
                        })
                        .collect(),
                })
                .collect(),
            primary_key: table
                .primary_key_columns()
                .into_iter()
                .flatten()
                .map(|c| IndexColumn {
                    name: c.as_column().name().into(),
                    length: c.length(),
                    sort_order: c.sort_order().map(|so| match so {
                        SQLSortOrder::Asc => SortOrder::Asc,
                        SQLSortOrder::Desc => SortOrder::Desc,
                    }),
                    operator_class: None,
                })
                .collect(),
            default_character_set: Some("utf8mb4".into()),
            collate: Some("utf8mb4_unicode_ci".into()),
        }
        .to_string()
    }

    fn render_drop_and_recreate_index(&self, indexes: Pair<IndexWalker<'_>>) -> Vec<String> {
        // Order matters: dropping the old index first wouldn't work when foreign key constraints are still relying on it.
        vec![
            self.render_create_index(indexes.next),
            sql_ddl::mysql::DropIndex {
                index_name: indexes.previous.name().into(),
                table_name: indexes.previous.table().name().into(),
            }
            .to_string(),
        ]
    }

    fn render_drop_enum(&self, _: EnumWalker<'_>) -> Vec<String> {
        unreachable!(
            "Unreachable render_drop_enum() on MySQL. enums are defined on each column that uses them on MySQL"
        )
    }

    fn render_drop_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {table} DROP FOREIGN KEY {constraint_name}",
            table = self.quote(foreign_key.table().name()),
            constraint_name = Quoted::mysql_ident(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, index: IndexWalker<'_>) -> String {
        sql_ddl::mysql::DropIndex {
            table_name: index.table().name().into(),
            index_name: index.name().into(),
        }
        .to_string()
    }

    fn render_drop_table(&self, _namespace: Option<&str>, table_name: &str) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_display(&sql_ddl::mysql::DropTable {
                    table_name: table_name.into(),
                })
            })
        })
    }

    fn render_redefine_tables(&self, _names: &[RedefineTable], _schemas: Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_redefine_table on MySQL")
    }

    fn render_rename_table(&self, _namespace: Option<&str>, name: &str, new_name: &str) -> String {
        sql_ddl::mysql::AlterTable {
            table_name: name.into(),
            changes: vec![sql_ddl::mysql::AlterTableClause::RenameTo {
                next_name: new_name.into(),
            }],
        }
        .to_string()
    }

    fn render_create_table(&self, table: TableWalker<'_>) -> String {
        self.render_create_table_as(table, QuotedWithPrefix(None, Quoted::mysql_ident(table.name())))
    }

    fn render_drop_view(&self, view: ViewWalker<'_>) -> String {
        format!("DROP VIEW {}", Quoted::mysql_ident(view.name()))
    }

    fn render_drop_user_defined_type(&self, _: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("render_drop_user_defined_type on MySQL")
    }

    fn render_rename_foreign_key(&self, _fks: Pair<ForeignKeyWalker<'_>>) -> String {
        unreachable!("render RenameForeignKey on MySQL")
    }
}

fn render_mysql_modify(
    changes: &ColumnChanges,
    new_default: Option<&sql_schema_describer::DefaultValue>,
    next_column: ColumnWalker<'_>,
) -> String {
    let column_type: Option<String> = if changes.type_changed() {
        Some(next_column.column_type().full_data_type.clone()).filter(|r| !r.is_empty() || r.contains("datetime"))
    // @default(now()) does not work with datetimes of certain sizes
    } else {
        Some(next_column.column_type().full_data_type.clone()).filter(|r| !r.is_empty())
    };

    let column_type = column_type
        .map(Cow::Owned)
        .unwrap_or_else(|| render_column_type(next_column));

    let default = new_default
        .filter(|default| !default.is_empty_dbgenerated())
        .map(|default| render_default(next_column, default))
        .filter(|expr| !expr.is_empty())
        .map(|expression| format!(" DEFAULT {}", expression))
        .unwrap_or_else(String::new);

    format!(
        "MODIFY {column_name} {column_type}{nullability}{default}{sequence}",
        column_name = Quoted::mysql_ident(&next_column.name()),
        column_type = column_type,
        nullability = if next_column.arity().is_required() {
            " NOT NULL"
        } else {
            " NULL"
        },
        default = default,
        sequence = if next_column.is_autoincrement() {
            " AUTO_INCREMENT"
        } else {
            ""
        },
    )
}

fn render_column_type(column: ColumnWalker<'_>) -> Cow<'static, str> {
    if let ColumnTypeFamily::Enum(enum_id) = column.column_type_family() {
        let variants: String = column.walk(*enum_id).values().map(Quoted::mysql_string).join(", ");

        return format!("ENUM({})", variants).into();
    }

    if let ColumnTypeFamily::Unsupported(description) = &column.column_type().family {
        return description.to_string().into();
    }

    let native_type = column
        .column_native_type()
        .expect("Column native type missing in mysql_renderer::render_column_type()");

    fn render(input: Option<u32>) -> String {
        match input {
            None => "".to_string(),
            Some(arg) => format!("({})", arg),
        }
    }

    fn render_decimal(input: Option<(u32, u32)>) -> String {
        match input {
            None => "".to_string(),
            Some((precision, scale)) => format!("({}, {})", precision, scale),
        }
    }

    match native_type {
        MySqlType::Int => "INTEGER".into(),
        MySqlType::SmallInt => "SMALLINT".into(),
        MySqlType::TinyInt if column.column_type_family().is_boolean() => "BOOLEAN".into(),
        MySqlType::TinyInt => "TINYINT".into(),
        MySqlType::MediumInt => "MEDIUMINT".into(),
        MySqlType::BigInt => "BIGINT".into(),
        MySqlType::Decimal(precision) => format!("DECIMAL{}", render_decimal(*precision)).into(),
        MySqlType::Float => "FLOAT".into(),
        MySqlType::Double => "DOUBLE".into(),
        MySqlType::Bit(size) => format!("BIT({size})", size = size).into(),
        MySqlType::Char(size) => format!("CHAR({size})", size = size).into(),
        MySqlType::VarChar(size) => format!("VARCHAR({size})", size = size).into(),
        MySqlType::Binary(size) => format!("BINARY({size})", size = size).into(),
        MySqlType::VarBinary(size) => format!("VARBINARY({size})", size = size).into(),
        MySqlType::TinyBlob => "TINYBLOB".into(),
        MySqlType::Blob => "BLOB".into(),
        MySqlType::MediumBlob => "MEDIUMBLOB".into(),
        MySqlType::LongBlob => "LONGBLOB".into(),
        MySqlType::TinyText => "TINYTEXT".into(),
        MySqlType::Text => "TEXT".into(),
        MySqlType::MediumText => "MEDIUMTEXT".into(),
        MySqlType::LongText => "LONGTEXT".into(),
        MySqlType::Date => "DATE".into(),
        MySqlType::Time(precision) => format!("TIME{}", render(*precision)).into(),
        MySqlType::DateTime(precision) => format!("DATETIME{}", render(*precision)).into(),
        MySqlType::Timestamp(precision) => format!("TIMESTAMP{}", render(*precision)).into(),
        MySqlType::Year => "YEAR".into(),
        MySqlType::Json => "JSON".into(),
        MySqlType::UnsignedInt => "INTEGER UNSIGNED".into(),
        MySqlType::UnsignedSmallInt => "SMALLINT UNSIGNED".into(),
        MySqlType::UnsignedTinyInt => "TINYINT UNSIGNED".into(),
        MySqlType::UnsignedMediumInt => "MEDIUMINT UNSIGNED".into(),
        MySqlType::UnsignedBigInt => "BIGINT UNSIGNED".into(),
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}

/// https://dev.mysql.com/doc/refman/8.0/en/alter-table.html
///
/// We don't use SET DEFAULT because it can't be used to set the default to an expression on most
/// MySQL versions. We use MODIFY for default changes instead.
#[derive(Debug)]
enum MysqlAlterColumn {
    DropDefault,
    Modify {
        new_default: Option<DefaultValue>,
        changes: ColumnChanges,
    },
}

impl MysqlAlterColumn {
    fn new(columns: Pair<ColumnWalker<'_>>, changes: ColumnChanges) -> Self {
        if changes.only_default_changed() && columns.next.default().is_none() {
            return MysqlAlterColumn::DropDefault;
        }

        let defaults = (
            columns.previous.default().as_ref().map(|d| d.kind()),
            columns.next.default().as_ref().map(|d| d.kind()),
        );

        // @default(dbgenerated()) does not give us the information in the prisma schema, so we have to
        // transfer it from the introspected current state of the database.
        let new_default = match defaults {
            (Some(DefaultKind::DbGenerated(Some(previous))), Some(DefaultKind::DbGenerated(next)))
                if (next.is_none() || next.as_deref() == Some("")) && !previous.is_empty() =>
            {
                Some(DefaultValue::db_generated(previous.clone()))
            }
            _ => columns.next.default().map(|d| d.inner()).cloned(),
        };

        MysqlAlterColumn::Modify { changes, new_default }
    }
}

fn render_default<'a>(column: ColumnWalker<'a>, default: &'a DefaultValue) -> Cow<'a, str> {
    match default.kind() {
        DefaultKind::DbGenerated(Some(val)) => val.as_str().into(),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            Quoted::mysql_string(escape_string_literal(val)).to_string().into()
        }
        DefaultKind::Now => {
            let precision = column
                .column_native_type()
                .and_then(MySqlType::timestamp_precision)
                .unwrap_or(3);

            format!("CURRENT_TIMESTAMP({})", precision).into()
        }
        DefaultKind::Value(PrismaValue::DateTime(dt)) if column.column_type_family().is_datetime() => {
            Quoted::mysql_string(dt.to_rfc3339()).to_string().into()
        }
        DefaultKind::Value(val) => val.to_string().into(),
        DefaultKind::DbGenerated(None) | DefaultKind::Sequence(_) | DefaultKind::UniqueRowid => unreachable!(),
    }
}
