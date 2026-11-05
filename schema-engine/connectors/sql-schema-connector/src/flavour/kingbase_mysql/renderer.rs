use super::schema_differ::explicit_cast_supported;
use crate::{
    flavour::MysqlRenderer,
    migration_pair::MigrationPair,
    sql_migration::{AlterColumn, AlterEnum, AlterTable, RedefineTable, TableChange},
    sql_renderer::{Quoted, QuotedWithPrefix, SqlRenderer},
    sql_schema_differ::ColumnChanges,
};
use itertools::Itertools;
use psl::builtin_connectors::KingbaseMySqlType;
use regex::Regex;
use sql_ddl::{IndexColumn, SortOrder, mysql as ddl};
use sql_schema_describer::{
    self as sql, ColumnTypeFamily, DefaultKind, DefaultValue, PrismaValue, SQLSortOrder, SqlSchema,
    walkers::{
        EnumWalker, ForeignKeyWalker, IndexWalker, TableColumnWalker, TableWalker, UserDefinedTypeWalker, ViewWalker,
    },
};
use std::{borrow::Cow, fmt::Write as _, sync::LazyLock};

/// Kingbase MySQL migration renderer.
///
/// Reuses MySQL DDL builders while keeping Kingbase-specific syntax and
/// identifier escaping separate from standard MySQL output.
#[derive(Debug, Default)]
pub(crate) struct KingbaseMysqlRenderer;

impl SqlRenderer for KingbaseMysqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::KingbaseMysqlIdentifier(name)
    }

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        ddl::AlterTable {
            table_name: escape_identifier(foreign_key.table().name()),
            changes: vec![ddl::AlterTableClause::AddForeignKey(ddl::ForeignKey {
                constraint_name: foreign_key.constraint_name().map(escape_identifier),
                constrained_columns: foreign_key
                    .constrained_columns()
                    .map(|column| escape_identifier(column.name()))
                    .collect(),
                referenced_table: escape_identifier(foreign_key.referenced_table().name()),
                referenced_columns: foreign_key
                    .referenced_columns()
                    .map(|column| escape_identifier(column.name()))
                    .collect(),
                on_delete: Some(render_foreign_key_action(foreign_key.on_delete_action())),
                on_update: Some(render_foreign_key_action(foreign_key.on_update_action())),
            })],
        }
        .to_string()
    }

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        MysqlRenderer.render_alter_enum(alter_enum, schemas)
    }

    fn render_rename_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        let mut statements = vec![format!(
            "ALTER INDEX {} RENAME TO {}",
            self.quote(physical_index_name(indexes.previous)),
            self.quote(physical_index_name(indexes.next)),
        )];
        if let Some(comment) = index_logical_name_comment(indexes.next) {
            statements.push(comment);
        }
        statements
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let AlterTable { table_ids, changes } = alter_table;
        let tables = schemas.walk(*table_ids);
        let mut lines = Vec::new();

        for change in changes {
            match change {
                TableChange::DropPrimaryKey => {
                    let primary_key = tables.previous.primary_key().unwrap();
                    let ext: &sql::kingbase_mysql::KingbaseMysqlSchemaExt =
                        tables.previous.schema.downcast_connector_data();
                    let name = ext
                        .primary_key_constraint_names
                        .get(&primary_key.id)
                        .expect("Kingbase MySQL primary key constraint name is missing");
                    lines.push(format!("DROP CONSTRAINT {}", self.quote(name)));
                }
                TableChange::RenamePrimaryKey => unreachable!("Kingbase MySQL does not support renaming primary keys"),
                TableChange::AddPrimaryKey => lines.push(format!(
                    "ADD PRIMARY KEY ({})",
                    tables
                        .next
                        .primary_key_columns()
                        .unwrap()
                        .map(|column| {
                            let mut rendered = self.quote(column.as_column().name()).to_string();

                            if let Some(length) = column.length() {
                                write!(rendered, "({length})").unwrap();
                            }

                            if let Some(sort_order) = column.sort_order().filter(|order| *order != SQLSortOrder::Asc) {
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
                    lines.push(format!("ADD COLUMN {}", render_column(column)));
                }
                TableChange::DropColumn { column_id } => {
                    let column = tables.previous.walk(*column_id);
                    lines.push(
                        sql_ddl::mysql::AlterTableClause::DropColumn {
                            column_name: escape_identifier(column.name()),
                        }
                        .to_string(),
                    );
                }
                TableChange::AlterColumn(AlterColumn {
                    changes,
                    column_id,
                    type_change,
                }) => {
                    let columns = schemas.walk(*column_id);
                    if changes.only_default_changed() && columns.next.default().is_none() {
                        lines.push(format!(
                            "ALTER COLUMN {} DROP DEFAULT",
                            Quoted::KingbaseMysqlIdentifier(columns.previous.name())
                        ));
                    } else {
                        let defaults = (
                            columns.previous.default().as_ref().map(|default| default.kind()),
                            columns.next.default().as_ref().map(|default| default.kind()),
                        );
                        let new_default = match defaults {
                            (Some(DefaultKind::DbGenerated(Some(previous))), Some(DefaultKind::DbGenerated(next)))
                                if (next.is_none() || next.as_deref() == Some("")) && !previous.is_empty() =>
                            {
                                Some(DefaultValue::db_generated(previous.clone()))
                            }
                            _ => columns.next.default().map(|default| default.inner()).cloned(),
                        };

                        if matches!(type_change, Some(crate::sql_migration::ColumnTypeChange::RiskyCast))
                            && explicit_cast_supported(
                                columns.previous.column_native_type::<KingbaseMySqlType>(),
                                columns.next.column_native_type::<KingbaseMySqlType>(),
                            )
                        {
                            lines.extend(render_alter_column_with_using(changes, new_default.as_ref(), columns));
                        } else {
                            lines.push(render_modify_column(changes, new_default.as_ref(), columns.next));
                        }
                    }
                }
                TableChange::DropAndRecreateColumn { column_id, changes: _ } => {
                    let columns = schemas.walk(*column_id);
                    lines.push(format!("DROP COLUMN {}", quote_identifier(columns.previous.name())));
                    lines.push(format!("ADD COLUMN {}", render_column(columns.next)));
                }
            }
        }

        if lines.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "ALTER TABLE {} {}",
                self.quote(tables.previous.name()),
                lines.join(",\n    ")
            )]
        }
    }

    fn render_create_enum(&self, create_enum: EnumWalker<'_>) -> Vec<String> {
        MysqlRenderer.render_create_enum(create_enum)
    }

    fn render_create_index(&self, index: IndexWalker<'_>) -> String {
        let mut rendered = if index.index_type() == sql::IndexType::Fulltext {
            render_fulltext_index(index)
        } else {
            ddl::CreateIndex {
                r#type: if index.is_unique() {
                    ddl::IndexType::Unique
                } else {
                    ddl::IndexType::Normal
                },
                index_name: escape_identifier(physical_index_name(index)),
                on: (
                    escape_identifier(index.table().name()),
                    index.columns().map(render_index_column).collect(),
                ),
            }
            .to_string()
        };
        if let Some(comment) = index_logical_name_comment(index) {
            write!(rendered, ";\n{comment}").unwrap();
        }
        rendered
    }

    fn render_create_table(&self, table: TableWalker<'_>) -> String {
        self.render_create_table_as(
            table,
            QuotedWithPrefix(None, Quoted::KingbaseMysqlIdentifier(table.name())),
        )
    }

    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        let kingbase_quote = |quoted| match quoted {
            Quoted::Backticks(name) => self.quote(name),
            quoted => quoted,
        };
        let table_name = QuotedWithPrefix(table_name.0.map(kingbase_quote), kingbase_quote(table_name.1));
        // Kingbase accepts the MySQL column and index syntax, but does not
        // accept MySQL's table-level character-set/collation options.
        let mut rendered = ddl::CreateTable {
            table_name: &table_name,
            columns: table.columns().map(render_column).collect(),
            // GIN expressions and descending unique indexes are created by
            // separate CREATE INDEX steps; other indexes stay inline.
            indexes: table
                .indexes()
                .filter(|index| !index.is_primary_key() && !super::index_is_created_separately(*index))
                .map(|index| ddl::IndexClause {
                    index_name: Some(escape_identifier(physical_index_name(index))),
                    r#type: match index.index_type() {
                        sql::IndexType::Unique => ddl::IndexType::Unique,
                        sql::IndexType::Normal => ddl::IndexType::Normal,
                        sql::IndexType::Fulltext | sql::IndexType::PrimaryKey => unreachable!(),
                    },
                    columns: index.columns().map(render_index_column).collect(),
                })
                .collect(),
            primary_key: table
                .primary_key_columns()
                .into_iter()
                .flatten()
                .map(render_primary_key_column)
                .collect(),
            default_character_set: None,
            collate: None,
        }
        .to_string();
        for index in table
            .indexes()
            .filter(|index| !index.is_primary_key() && !super::index_is_created_separately(*index))
        {
            if let Some(comment) = index_logical_name_comment(index) {
                write!(rendered, ";\n{comment}").unwrap();
            }
        }
        rendered
    }

    fn render_drop_and_recreate_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        if indexes.previous.index_type() == sql::IndexType::Fulltext
            || indexes.next.index_type() == sql::IndexType::Fulltext
        {
            return vec![
                self.render_drop_index(None, indexes.previous),
                self.render_create_index(indexes.next),
            ];
        }

        // Keep the replacement available before removing the old index. Use
        // Kingbase's ownership-aware deletion, including constraint-backed
        // unique indexes, instead of MySQL's DROP INDEX ... ON table syntax.
        vec![
            self.render_create_index(indexes.next),
            self.render_drop_index(None, indexes.previous),
        ]
    }

    fn render_drop_enum(&self, namespace: Option<&str>, dropped_enum: EnumWalker<'_>) -> Vec<String> {
        MysqlRenderer.render_drop_enum(namespace, dropped_enum)
    }

    fn render_drop_foreign_key(&self, namespace: Option<&str>, foreign_key: ForeignKeyWalker<'_>) -> String {
        let _ = namespace;
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = self.quote(foreign_key.table().name()),
            constraint_name = Quoted::KingbaseMysqlIdentifier(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, namespace: Option<&str>, index: IndexWalker<'_>) -> String {
        let _ = namespace;
        let ext: &sql::kingbase_mysql::KingbaseMysqlSchemaExt = index.schema.downcast_connector_data();
        if let Some(constraint_name) = ext.unique_constraint_names.get(&index.id) {
            format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                self.quote(index.table().name()),
                self.quote(constraint_name),
            )
        } else {
            format!("DROP INDEX {}", self.quote(physical_index_name(index)))
        }
    }

    fn render_drop_table(&self, namespace: Option<&str>, table_name: &str) -> Vec<String> {
        let _ = namespace;
        vec![
            ddl::DropTable {
                table_name: escape_identifier(table_name),
            }
            .to_string(),
        ]
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        MysqlRenderer.render_redefine_tables(tables, schemas)
    }

    fn render_rename_table(&self, namespace: Option<&str>, name: &str, new_name: &str) -> String {
        let _ = namespace;
        format!("ALTER TABLE {} RENAME TO {}", self.quote(name), self.quote(new_name))
    }

    fn render_drop_view(&self, namespace: Option<&str>, view: ViewWalker<'_>) -> String {
        let _ = namespace;
        format!("DROP VIEW {}", self.quote(view.name()))
    }

    fn render_drop_user_defined_type(&self, namespace: Option<&str>, udt: &UserDefinedTypeWalker<'_>) -> String {
        MysqlRenderer.render_drop_user_defined_type(namespace, udt)
    }

    fn render_rename_foreign_key(&self, fks: MigrationPair<ForeignKeyWalker<'_>>) -> String {
        MysqlRenderer.render_rename_foreign_key(fks)
    }

    fn render_create_namespace(&self, namespace: sql::NamespaceWalker<'_>) -> Vec<String> {
        MysqlRenderer.render_create_namespace(namespace)
    }
}

// sql-ddl::mysql deliberately retains upstream identifier handling. Escape only
// the raw identifier fields passed to its builders by the Kingbase renderer.
fn escape_identifier(name: &str) -> Cow<'_, str> {
    if name.contains('`') {
        Cow::Owned(name.replace('`', "``"))
    } else {
        Cow::Borrowed(name)
    }
}

fn render_foreign_key_action(action: sql::ForeignKeyAction) -> ddl::ForeignKeyAction {
    match action {
        sql::ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
        sql::ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
        sql::ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
        sql::ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
        sql::ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
    }
}

fn physical_index_name(index: IndexWalker<'_>) -> &str {
    let ext: &sql::kingbase_mysql::KingbaseMysqlSchemaExt = index.schema.downcast_connector_data();
    ext.physical_index_names
        .get(&index.id)
        .map(String::as_str)
        .unwrap_or_else(|| index.name())
}

fn index_logical_name_comment(index: IndexWalker<'_>) -> Option<String> {
    (physical_index_name(index) != index.name()).then(|| {
        format!(
            "COMMENT ON INDEX {} IS '{}'",
            Quoted::KingbaseMysqlIdentifier(physical_index_name(index)),
            format!("{}{}", sql::kingbase_mysql::INDEX_LOGICAL_NAME_PREFIX, index.name()).replace('\'', "''"),
        )
    })
}

fn render_fulltext_index(index: IndexWalker<'_>) -> String {
    let document = index
        .columns()
        .map(|column| {
            format!(
                "COALESCE({}, '')",
                Quoted::KingbaseMysqlIdentifier(column.as_column().name())
            )
        })
        .reduce(|document, column| format!("textcat({document}, textcat(' ', {column}))"))
        .expect("a full-text index must contain at least one column");

    format!(
        "CREATE INDEX {index_name} ON {table_name} USING GIN (to_tsvector('simple', {document}))",
        index_name = Quoted::KingbaseMysqlIdentifier(physical_index_name(index)),
        table_name = Quoted::KingbaseMysqlIdentifier(index.table().name()),
    )
}

fn render_column(col: TableColumnWalker<'_>) -> ddl::Column<'_> {
    let default = col
        .default()
        .filter(|default| {
            !matches!(
                default.kind(),
                DefaultKind::Sequence(_) | DefaultKind::DbGenerated(None)
            ) && !matches!(col.column_type_family(), ColumnTypeFamily::Json)
                && !matches!(col.column_type_family(), ColumnTypeFamily::Binary if !default.is_db_generated())
        })
        .map(|default| render_default(col, default.inner()));

    ddl::Column {
        column_name: escape_identifier(col.name()),
        not_null: col.arity().is_required(),
        column_type: render_column_type(col),
        default,
        auto_increment: col.is_autoincrement(),
        ..Default::default()
    }
}

fn render_primary_key_column(column: sql::walkers::IndexColumnWalker<'_>) -> IndexColumn<'_> {
    let mut rendered = render_index_column(column);
    // The describer records the default ascending order explicitly, while
    // Kingbase's PRIMARY KEY column syntax requires omitting that marker.
    if matches!(rendered.sort_order, Some(SortOrder::Asc)) {
        rendered.sort_order = None;
    }
    rendered
}

fn render_index_column(column: sql::walkers::IndexColumnWalker<'_>) -> IndexColumn<'_> {
    IndexColumn {
        name: escape_identifier(column.as_column().name()),
        length: column.length(),
        sort_order: column.sort_order().map(|sort_order| match sort_order {
            SQLSortOrder::Asc => SortOrder::Asc,
            SQLSortOrder::Desc => SortOrder::Desc,
        }),
        operator_class: None,
    }
}

fn render_column_type(column: TableColumnWalker<'_>) -> Cow<'static, str> {
    if let ColumnTypeFamily::Enum(enum_id) = column.column_type_family() {
        let variants = column.walk(*enum_id).values().map(Quoted::mysql_string).join(", ");
        return format!("ENUM({variants})").into();
    }

    if let ColumnTypeFamily::Unsupported(description) = &column.column_type().family {
        return description.to_string().into();
    }

    let native_type = column
        .column_native_type::<KingbaseMySqlType>()
        .expect("Column native type missing in kingbase_mysql_renderer::render_column_type()");

    fn render(input: Option<u32>) -> String {
        input.map_or_else(String::new, |arg| format!("({arg})"))
    }

    fn render_decimal(input: Option<(u32, u32)>) -> String {
        input.map_or_else(String::new, |(precision, scale)| format!("({precision}, {scale})"))
    }

    match native_type {
        KingbaseMySqlType::Int => "INTEGER".into(),
        KingbaseMySqlType::SmallInt => "SMALLINT".into(),
        KingbaseMySqlType::TinyInt | KingbaseMySqlType::UnsignedTinyInt if column.column_type_family().is_boolean() => {
            "BOOLEAN".into()
        }
        KingbaseMySqlType::TinyInt => "TINYINT".into(),
        KingbaseMySqlType::MediumInt => "MEDIUMINT".into(),
        KingbaseMySqlType::BigInt => "BIGINT".into(),
        KingbaseMySqlType::Decimal(precision) => format!("DECIMAL{}", render_decimal(*precision)).into(),
        KingbaseMySqlType::Float => "FLOAT".into(),
        KingbaseMySqlType::Double => "DOUBLE".into(),
        KingbaseMySqlType::Bit(size) => format!("BIT({size})").into(),
        KingbaseMySqlType::Char(size) => format!("CHAR({size})").into(),
        KingbaseMySqlType::VarChar(size) => format!("VARCHAR({size})").into(),
        KingbaseMySqlType::Binary(size) => format!("BINARY({size})").into(),
        KingbaseMySqlType::VarBinary(size) => format!("VARBINARY({size})").into(),
        KingbaseMySqlType::TinyBlob => "TINYBLOB".into(),
        KingbaseMySqlType::Blob => "BLOB".into(),
        KingbaseMySqlType::MediumBlob => "MEDIUMBLOB".into(),
        KingbaseMySqlType::LongBlob => "LONGBLOB".into(),
        KingbaseMySqlType::TinyText => "TINYTEXT".into(),
        KingbaseMySqlType::Text => "TEXT".into(),
        KingbaseMySqlType::MediumText => "MEDIUMTEXT".into(),
        KingbaseMySqlType::LongText => "LONGTEXT".into(),
        KingbaseMySqlType::Date => "DATE".into(),
        KingbaseMySqlType::Time(precision) => format!("TIME{}", render(*precision)).into(),
        KingbaseMySqlType::DateTime(precision) => format!("DATETIME{}", render(*precision)).into(),
        KingbaseMySqlType::Timestamp(precision) => format!("TIMESTAMP{}", render(*precision)).into(),
        KingbaseMySqlType::Year => "YEAR".into(),
        KingbaseMySqlType::Json => "JSON".into(),
        // Use MySQL DDL syntax; the describer handles the uint4/uint8 catalog names.
        KingbaseMySqlType::UnsignedInt => "INTEGER UNSIGNED".into(),
        KingbaseMySqlType::UnsignedSmallInt
        | KingbaseMySqlType::UnsignedTinyInt
        | KingbaseMySqlType::UnsignedMediumInt => {
            unreachable!("small unsigned native types are rejected by validation")
        }
        KingbaseMySqlType::UnsignedBigInt => "BIGINT UNSIGNED".into(),
    }
}

fn render_modify_column(
    changes: &ColumnChanges,
    new_default: Option<&DefaultValue>,
    next_column: TableColumnWalker<'_>,
) -> String {
    let column_type = if changes.type_changed() {
        Some(next_column.column_type().full_data_type.clone())
            .filter(|type_name| !type_name.is_empty() || type_name.to_ascii_lowercase().contains("datetime"))
    } else {
        Some(next_column.column_type().full_data_type.clone()).filter(|type_name| !type_name.is_empty())
    }
    .map(Cow::Owned)
    .unwrap_or_else(|| render_column_type(next_column));

    let default = new_default
        .filter(|default| !default.is_empty_dbgenerated())
        .map(|default| render_default(next_column, default))
        .filter(|expression| !expression.is_empty())
        .map(|expression| format!(" DEFAULT {expression}"))
        .unwrap_or_default();

    format!(
        "MODIFY {column_name} {column_type}{nullability}{default}{sequence}",
        column_name = Quoted::KingbaseMysqlIdentifier(next_column.name()),
        nullability = if next_column.arity().is_required() {
            " NOT NULL"
        } else {
            " NULL"
        },
        sequence = if next_column.is_autoincrement() {
            " AUTO_INCREMENT"
        } else {
            ""
        },
    )
}

fn render_alter_column_with_using(
    changes: &ColumnChanges,
    new_default: Option<&DefaultValue>,
    columns: MigrationPair<TableColumnWalker<'_>>,
) -> Vec<String> {
    let column_name = Quoted::KingbaseMysqlIdentifier(columns.previous.name());
    let target_type = render_column_type(columns.next);
    let using = render_using_expression(columns).expect("explicit cast support must have a USING expression");
    let mut clauses = vec![format!("ALTER COLUMN {column_name} TYPE {target_type} USING ({using})",)];

    if changes.default_changed() {
        match new_default {
            Some(default) if !default.is_empty_dbgenerated() => clauses.push(format!(
                "ALTER COLUMN {column_name} SET DEFAULT {default}",
                default = render_default(columns.next, default),
            )),
            _ => clauses.push(format!("ALTER COLUMN {column_name} DROP DEFAULT")),
        }
    }

    if changes.arity_changed() {
        clauses.push(if columns.next.arity().is_required() {
            format!("ALTER COLUMN {column_name} SET NOT NULL")
        } else {
            format!("ALTER COLUMN {column_name} DROP NOT NULL")
        });
    }

    clauses
}

fn render_using_expression(columns: MigrationPair<TableColumnWalker<'_>>) -> Option<String> {
    let previous = columns.previous.column_native_type::<KingbaseMySqlType>()?;
    let next = columns.next.column_native_type::<KingbaseMySqlType>()?;
    let column = Quoted::KingbaseMysqlIdentifier(columns.previous.name()).to_string();
    let target_type = render_column_type(columns.next);

    render_using_expression_for_types(previous, next, &column, &target_type)
}

fn render_using_expression_for_types(
    previous: &KingbaseMySqlType,
    next: &KingbaseMySqlType,
    column: &str,
    target_type: &str,
) -> Option<String> {
    if !explicit_cast_supported(Some(previous), Some(next)) {
        return None;
    }

    let expression = if is_numeric(previous) && matches!(next, KingbaseMySqlType::Bit(_)) {
        format!("({column})::bigint::{target_type}")
    } else if matches!(previous, KingbaseMySqlType::Bit(_)) {
        format!("({column})::text::{target_type}")
    } else if is_binary(previous) && !is_binary(next) {
        format!("convert_from(({column})::bytea, 'UTF8')::{target_type}")
    } else if (is_json(previous) && !is_string(next)) || (is_binary(next) && !is_binary(previous)) {
        format!("({column})::text::{target_type}")
    } else if is_json(next) && is_numeric(previous) {
        format!("to_json({column})")
    } else if is_json(next) {
        format!("({column})::text::{target_type}")
    } else {
        format!("{column}::{target_type}")
    };

    Some(expression)
}

fn is_binary(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Bit(_)
            | KingbaseMySqlType::Binary(_)
            | KingbaseMySqlType::VarBinary(_)
            | KingbaseMySqlType::TinyBlob
            | KingbaseMySqlType::Blob
            | KingbaseMySqlType::MediumBlob
            | KingbaseMySqlType::LongBlob
    )
}

fn is_string(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Char(_)
            | KingbaseMySqlType::VarChar(_)
            | KingbaseMySqlType::TinyText
            | KingbaseMySqlType::Text
            | KingbaseMySqlType::MediumText
            | KingbaseMySqlType::LongText
    )
}

fn is_numeric(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Int
            | KingbaseMySqlType::UnsignedInt
            | KingbaseMySqlType::SmallInt
            | KingbaseMySqlType::UnsignedSmallInt
            | KingbaseMySqlType::TinyInt
            | KingbaseMySqlType::UnsignedTinyInt
            | KingbaseMySqlType::MediumInt
            | KingbaseMySqlType::UnsignedMediumInt
            | KingbaseMySqlType::BigInt
            | KingbaseMySqlType::UnsignedBigInt
            | KingbaseMySqlType::Decimal(_)
            | KingbaseMySqlType::Float
            | KingbaseMySqlType::Double
            | KingbaseMySqlType::Year
    )
}

fn is_json(native_type: &KingbaseMySqlType) -> bool {
    matches!(native_type, KingbaseMySqlType::Json)
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"'"#).unwrap());
    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn render_default<'a>(column: TableColumnWalker<'a>, default: &'a DefaultValue) -> Cow<'a, str> {
    match default.kind() {
        DefaultKind::DbGenerated(Some(value)) => value.as_str().into(),
        DefaultKind::Value(PrismaValue::String(value)) | DefaultKind::Value(PrismaValue::Enum(value)) => {
            Quoted::mysql_string(escape_string_literal(value)).to_string().into()
        }
        DefaultKind::Now => column
            .column_native_type::<KingbaseMySqlType>()
            .and_then(KingbaseMySqlType::timestamp_precision)
            .map(|precision| format!("CURRENT_TIMESTAMP({precision})").into())
            .unwrap_or_else(|| "CURRENT_TIMESTAMP".into()),
        DefaultKind::Value(PrismaValue::DateTime(value)) if column.column_type_family().is_datetime() => {
            Quoted::mysql_string(value.to_rfc3339()).to_string().into()
        }
        DefaultKind::Value(value) => value.to_string().into(),
        DefaultKind::DbGenerated(None) | DefaultKind::Sequence(_) | DefaultKind::UniqueRowid => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_default_foreign_key_actions_are_rendered() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        let dialect = crate::SqlSchemaDialect::kingbase_mysql();
        let source = "datasource db {\n provider = \"kingbase-mysql\"\n }\n";
        let target = format!(
            "{source}\nmodel Parent {{\n id Int @id\n children Child[]\n }}\n\
             model Child {{\n id Int @id\n parentId Int @default(1)\n \
             parent Parent @relation(fields: [parentId], references: [id], onDelete: SetDefault, onUpdate: SetDefault, map: \"child_parent_fk\")\n }}"
        );
        let calculate = |schema: String| {
            dialect
                .schema_from_datamodel(
                    vec![("schema.prisma".into(), schema.into())],
                    Some("public"),
                    &NoExtensionTypes,
                )
                .unwrap()
        };
        let migration = dialect.diff(calculate(source.to_owned()), calculate(target), &Default::default());
        let script =
            crate::apply_migration::render_script(&migration, &Default::default(), &KingbaseMysqlRenderer).unwrap();
        assert!(script.contains(
            "ALTER TABLE `Child` ADD CONSTRAINT `child_parent_fk` FOREIGN KEY (`parentId`) REFERENCES `Parent`(`id`) ON DELETE SET DEFAULT ON UPDATE SET DEFAULT;"
        ), "{script}");
    }

    fn index_schema(name: &str, kind: sql::IndexType, owner: Option<&str>) -> SqlSchema {
        let mut schema = SqlSchema::default();
        let namespace = schema.push_namespace("public".into());
        let table = schema.push_table("items".into(), namespace, None);
        let column = schema.push_table_column(
            table,
            sql::Column {
                name: "value".into(),
                tpe: sql::ColumnType::pure(ColumnTypeFamily::String, sql::ColumnArity::Required),
                auto_increment: false,
                description: None,
            },
        );
        let index = schema.push_index_of_type(table, name.into(), kind);
        schema.push_index_column(sql::IndexColumn {
            index_id: index,
            column_id: column,
            sort_order: None,
            length: None,
        });
        let mut ext = sql::kingbase_mysql::KingbaseMysqlSchemaExt::default();
        if let Some(owner) = owner {
            ext.unique_constraint_names.insert(index, owner.into());
        }
        schema.set_connector_data(Box::new(ext));
        schema
    }

    #[test]
    fn introspected_primary_key_ascending_order_is_implicit_in_ddl() {
        let mut schema = SqlSchema::default();
        let namespace = schema.push_namespace("public".into());
        let table = schema.push_table("items".into(), namespace, None);
        let mut tpe = sql::ColumnType::pure(ColumnTypeFamily::String, sql::ColumnArity::Required);
        tpe.native_type = Some(psl::datamodel_connector::NativeTypeInstance::new(
            KingbaseMySqlType::VarChar(32),
        ));
        let column = schema.push_table_column(
            table,
            sql::Column {
                name: "value".into(),
                tpe,
                auto_increment: false,
                description: None,
            },
        );
        let index = schema.push_primary_key(table, "items_pkey".into());
        schema.push_index_column(sql::IndexColumn {
            index_id: index,
            column_id: column,
            sort_order: Some(SQLSortOrder::Asc),
            length: None,
        });
        schema.set_connector_data(Box::<sql::kingbase_mysql::KingbaseMysqlSchemaExt>::default());
        let table = schema.table_walkers().next().unwrap();
        let created = KingbaseMysqlRenderer.render_create_table(table);
        assert!(created.contains("PRIMARY KEY (`value`)"), "{created}");
        assert!(!created.contains("`value` ASC"), "{created}");
        let alter = AlterTable {
            table_ids: MigrationPair::new(table.id, table.id),
            changes: vec![TableChange::AddPrimaryKey],
        };
        assert_eq!(
            KingbaseMysqlRenderer.render_alter_table(&alter, MigrationPair::new(&schema, &schema)),
            vec!["ALTER TABLE `items` ADD PRIMARY KEY (`value`)".to_owned()]
        );
    }

    // The regular differ emits RenameIndex. Force the otherwise dormant step
    // through the real script renderer rather than testing only rename SQL.
    fn forced_rebuild_script(before: SqlSchema, after: SqlSchema, old: &str, new: &str) -> String {
        let index = MigrationPair::new(
            before
                .table_walkers()
                .flat_map(|table| table.indexes())
                .find(|index| index.name() == old)
                .unwrap()
                .id,
            after
                .table_walkers()
                .flat_map(|table| table.indexes())
                .find(|index| index.name() == new)
                .unwrap()
                .id,
        );
        let migration = schema_connector::Migration::new(crate::sql_migration::SqlMigration {
            before,
            after,
            steps: vec![crate::sql_migration::SqlMigrationStep::RedefineIndex { index }],
        });
        crate::apply_migration::render_script(&migration, &Default::default(), &KingbaseMysqlRenderer).unwrap()
    }

    #[test]
    fn rebuild_step_uses_kingbase_index_and_constraint_deletion() {
        for (kind, owner, expected_drop) in [
            (sql::IndexType::Normal, None, "DROP INDEX `old_index`;"),
            (sql::IndexType::Unique, None, "DROP INDEX `old_index`;"),
            (
                sql::IndexType::Unique,
                Some("actual_owner"),
                "ALTER TABLE `items` DROP CONSTRAINT `actual_owner`;",
            ),
        ] {
            let script = forced_rebuild_script(
                index_schema("old_index", kind, owner),
                index_schema("new_index", kind, None),
                "old_index",
                "new_index",
            );
            let create = if kind == sql::IndexType::Unique {
                "CREATE UNIQUE INDEX `new_index` ON `items`(`value`);"
            } else {
                "CREATE INDEX `new_index` ON `items`(`value`);"
            };
            assert_eq!(script, format!("-- RedefineIndex\n{create}\n{expected_drop}\n"));
        }
    }

    #[test]
    fn fulltext_rebuild_keeps_drop_before_create() {
        for (previous, next) in [
            (sql::IndexType::Fulltext, sql::IndexType::Fulltext),
            (sql::IndexType::Fulltext, sql::IndexType::Normal),
            (sql::IndexType::Normal, sql::IndexType::Fulltext),
        ] {
            let script = forced_rebuild_script(
                index_schema("old_index", previous, None),
                index_schema("new_index", next, None),
                "old_index",
                "new_index",
            );
            assert_eq!(
                script.contains("USING GIN"),
                next == sql::IndexType::Fulltext,
                "{script}"
            );
            assert!(script.find("DROP INDEX `old_index`").unwrap() < script.find("CREATE INDEX `new_index`").unwrap());
            assert!(!script.contains("DROP INDEX `old_index` ON"), "{script}");
        }
    }

    #[cfg(feature = "kingbase-mysql-native")]
    #[tokio::test]
    #[ignore = "requires an isolated database in KINGBASE_INDEX_REBUILD_TEST_URL"]
    async fn forced_index_rebuilds_execute_on_kingbase_mysql() {
        use quaint::{prelude::Queryable, single::Quaint};
        use schema_connector::SchemaDialect;
        use sql_schema_describer::SqlSchemaDescriberBackend;

        let url = std::env::var("KINGBASE_INDEX_REBUILD_TEST_URL").expect("isolated test database URL is required");
        let conn = Quaint::new(&url).await.unwrap();
        for ansi_quotes in [false, true] {
            conn.raw_cmd(if ansi_quotes {
                "BEGIN; SET sql_mode='ANSI_QUOTES'"
            } else {
                "BEGIN; SET sql_mode=''"
            })
            .await
            .unwrap();
            let mode = conn
                .query_raw("SELECT current_setting('sql_mode') AS mode", &[])
                .await
                .unwrap();
            assert_eq!(
                mode.first()
                    .unwrap()
                    .get("mode")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("ANSI_QUOTES"),
                ansi_quotes
            );
            for (number, kind) in ["normal", "unique", "constraint", "descending", "fulltext"]
                .into_iter()
                .enumerate()
            {
                let table = format!("rebuild_{number}");
                let old = format!("old_index_{number}");
                let new = format!("new_index_{number}");
                let table_q = KingbaseMysqlRenderer.quote(&table).to_string();
                let old_q = KingbaseMysqlRenderer.quote(&old).to_string();
                let new_q = KingbaseMysqlRenderer.quote(&new).to_string();
                let constraint = if kind == "constraint" {
                    format!(", CONSTRAINT {old_q} UNIQUE(value)")
                } else {
                    String::new()
                };
                conn.raw_cmd(&format!("CREATE TABLE {table_q}(id INT PRIMARY KEY,value VARCHAR(32) NOT NULL {constraint}); INSERT INTO {table_q} VALUES (1,'kept')")).await.unwrap();
                let create = match kind {
                    "normal" => format!("CREATE INDEX {old_q} ON {table_q}(value)"),
                    "unique" => format!("CREATE UNIQUE INDEX {old_q} ON {table_q}(value)"),
                    "descending" => format!("CREATE UNIQUE INDEX {old_q} ON {table_q}(value DESC)"),
                    "fulltext" => format!(
                        "CREATE INDEX {old_q} ON {table_q} USING GIN(to_tsvector('simple', COALESCE(value, '')))"
                    ),
                    _ => String::new(),
                };
                if !create.is_empty() {
                    conn.raw_cmd(&create).await.unwrap();
                }
                let describer = sql::kingbase_mysql::SqlSchemaDescriber::new(&conn, Default::default());
                let before = describer.describe(&["public"]).await.unwrap();
                conn.raw_cmd(&format!(
                    "SAVEPOINT target_schema; ALTER INDEX {old_q} RENAME TO {new_q}"
                ))
                .await
                .unwrap();
                let target = describer.describe(&["public"]).await.unwrap();
                conn.raw_cmd("ROLLBACK TO SAVEPOINT target_schema").await.unwrap();
                let script = forced_rebuild_script(before, target.clone(), &old, &new);
                assert!(!script.contains(&format!("DROP INDEX {old_q} ON")), "{script}");
                if kind == "constraint" {
                    assert!(script.contains("DROP CONSTRAINT"), "{script}");
                }
                conn.raw_cmd(&script).await.unwrap();
                let row = conn
                    .query_raw(&format!("SELECT value FROM {table_q} WHERE id=1"), &[])
                    .await
                    .unwrap();
                assert_eq!(row.first().unwrap().get("value").unwrap().as_str(), Some("kept"));
                let actual = describer.describe(&["public"]).await.unwrap();
                let index = actual
                    .table_walkers()
                    .flat_map(|table| table.indexes())
                    .find(|index| index.name() == new)
                    .unwrap();
                assert!(
                    !actual
                        .table_walkers()
                        .flat_map(|table| table.indexes())
                        .any(|index| index.name() == old)
                );
                if matches!(kind, "unique" | "constraint" | "descending") {
                    let error = conn
                        .raw_cmd(&format!("INSERT INTO {table_q} VALUES (2,'kept')"))
                        .await
                        .unwrap_err();
                    assert_eq!(error.original_code(), Some("23505"));
                    // A rejected insert aborts this transaction until rollback.
                    conn.raw_cmd("ROLLBACK TO SAVEPOINT target_schema").await.unwrap();
                    // Reapply the exact rebuild after restoring the old index.
                    conn.raw_cmd(&script).await.unwrap();
                } else if kind == "fulltext" {
                    assert_eq!(index.index_type(), sql::IndexType::Fulltext);
                }
                let actual = describer.describe(&["public"]).await.unwrap();
                let diff = crate::SqlSchemaDialect::kingbase_mysql().diff(
                    crate::SqlDatabaseSchema::from(target).into(),
                    crate::SqlDatabaseSchema::from(actual).into(),
                    &Default::default(),
                );
                assert!(
                    diff.downcast_ref::<crate::sql_migration::SqlMigration>()
                        .steps
                        .is_empty(),
                    "{kind}: expected no subsequent migration"
                );
            }
            conn.raw_cmd("ROLLBACK").await.unwrap();
        }
    }

    #[test]
    fn starts_with_mysql_identifier_quoting() {
        assert_eq!(KingbaseMysqlRenderer.quote("users").to_string(), "`users`");
        assert_eq!(KingbaseMysqlRenderer.quote("old`name").to_string(), "`old``name`");
        assert_eq!(quote_identifier("old`name"), "`old``name`");
        assert_eq!(MysqlRenderer.quote("old`name").to_string(), "`old`name`");
        assert_eq!(Quoted::postgres_ident("old\"name").to_string(), "\"old\"name\"");
    }

    #[test]
    fn shared_mysql_ddl_builders_escape_only_kingbase_identifier_inputs() {
        let mut schema = SqlSchema::default();
        let namespace = schema.push_namespace("public".into());
        let table = schema.push_table("ta`ble".into(), namespace, None);
        let mut tpe = sql::ColumnType::pure(ColumnTypeFamily::Int, sql::ColumnArity::Required);
        tpe.native_type = Some(psl::datamodel_connector::NativeTypeInstance::new(
            KingbaseMySqlType::Int,
        ));
        let column = schema.push_table_column(
            table,
            sql::Column {
                name: "co`l".into(),
                tpe,
                auto_increment: false,
                description: None,
            },
        );
        let index = schema.push_index_of_type(table, "i`x".into(), sql::IndexType::Normal);
        schema.push_index_column(sql::IndexColumn {
            index_id: index,
            column_id: column,
            sort_order: None,
            length: None,
        });
        let foreign_key = schema.push_foreign_key(
            Some("f`k".into()),
            [table, table],
            [sql::ForeignKeyAction::Cascade, sql::ForeignKeyAction::Restrict],
        );
        schema.push_foreign_key_column(foreign_key, [column, column]);
        schema.set_connector_data(Box::<sql::kingbase_mysql::KingbaseMysqlSchemaExt>::default());
        let renderer = KingbaseMysqlRenderer;
        assert_eq!(
            renderer.render_create_table(schema.walk(table)),
            "CREATE TABLE `ta``ble` (\n    `co``l` INTEGER NOT NULL,\n\n    INDEX `i``x`(`co``l`)\n)"
        );
        assert_eq!(
            renderer.render_create_index(schema.walk(index)),
            "CREATE INDEX `i``x` ON `ta``ble`(`co``l`)"
        );
        assert_eq!(
            renderer.render_add_foreign_key(schema.walk(foreign_key)),
            "ALTER TABLE `ta``ble` ADD CONSTRAINT `f``k` FOREIGN KEY (`co``l`) REFERENCES `ta``ble`(`co``l`) ON DELETE CASCADE ON UPDATE RESTRICT"
        );
        assert_eq!(renderer.render_drop_table(None, "ta`ble"), vec!["DROP TABLE `ta``ble`"]);
        assert_eq!(
            renderer.render_rename_table(None, "ta`ble", "new`name"),
            "ALTER TABLE `ta``ble` RENAME TO `new``name`"
        );
        let alter = AlterTable {
            table_ids: MigrationPair::new(table, table),
            changes: vec![TableChange::DropColumn { column_id: column }],
        };
        assert_eq!(
            renderer.render_alter_table(&alter, MigrationPair::new(&schema, &schema)),
            vec!["ALTER TABLE `ta``ble` DROP COLUMN `co``l`"]
        );

        assert_eq!(
            ddl::DropTable {
                table_name: "ta`ble".into()
            }
            .to_string(),
            "DROP TABLE `ta`ble`"
        );
    }

    #[test]
    fn renders_using_expressions_for_non_implicit_casts() {
        assert_eq!(
            render_using_expression_for_types(
                &KingbaseMySqlType::DateTime(Some(0)),
                &KingbaseMySqlType::Double,
                "`created_at`",
                "DOUBLE",
            ),
            Some("`created_at`::DOUBLE".to_owned())
        );
        assert_eq!(
            render_using_expression_for_types(
                &KingbaseMySqlType::VarChar(32),
                &KingbaseMySqlType::Json,
                "`payload`",
                "JSON",
            ),
            Some("(`payload`)::text::JSON".to_owned())
        );
        assert_eq!(
            render_using_expression_for_types(
                &KingbaseMySqlType::Blob,
                &KingbaseMySqlType::VarChar(20),
                "`payload`",
                "VARCHAR(20)",
            ),
            Some("convert_from((`payload`)::bytea, 'UTF8')::VARCHAR(20)".to_owned())
        );
        assert_eq!(
            render_using_expression_for_types(
                &KingbaseMySqlType::BigInt,
                &KingbaseMySqlType::Bit(54),
                "`value`",
                "BIT(54)",
            ),
            Some("(`value`)::bigint::BIT(54)".to_owned())
        );
        assert_eq!(
            render_using_expression_for_types(&KingbaseMySqlType::Bit(32), &KingbaseMySqlType::Text, "`value`", "TEXT",),
            Some("(`value`)::text::TEXT".to_owned())
        );
    }

    #[test]
    fn does_not_render_using_for_unrepresentable_casts() {
        assert_eq!(
            render_using_expression_for_types(
                &KingbaseMySqlType::Blob,
                &KingbaseMySqlType::Int,
                "`payload`",
                "INTEGER",
            ),
            None
        );
    }
}
