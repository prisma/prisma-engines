use crate::common::{Indented, IndexColumn, IteratorJoin, SQL_INDENTATION};
use std::{
    borrow::Cow,
    fmt::{Display, Write as _},
};

struct Ident<'a>(&'a str);

impl Display for Ident<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("`")?;
        f.write_str(self.0)?;
        f.write_str("`")
    }
}

#[derive(Debug, Default)]
pub struct AlterTable<'a> {
    pub table_name: Cow<'a, str>,
    pub changes: Vec<AlterTableClause<'a>>,
}

impl Display for AlterTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ALTER TABLE ")?;
        Display::fmt(&Ident(self.table_name.as_ref()), f)?;

        if self.changes.len() == 1 {
            f.write_str(" ")?;
            Display::fmt(&self.changes[0], f)?;

            return Ok(());
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum AlterTableClause<'a> {
    AddForeignKey(ForeignKey<'a>),
    DropColumn {
        column_name: Cow<'a, str>,
    },
    DropForeignKey {
        constraint_name: Cow<'a, str>,
    },
    DropPrimaryKey,
    RenameIndex {
        previous_name: Cow<'a, str>,
        next_name: Cow<'a, str>,
    },
    RenameTo {
        next_name: Cow<'a, str>,
    },
}

impl Display for AlterTableClause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlterTableClause::RenameTo { next_name } => write!(f, "RENAME TO {next_name}"),
            AlterTableClause::RenameIndex {
                previous_name,
                next_name,
            } => write!(f, "RENAME INDEX `{previous_name}` TO `{next_name}`"),
            AlterTableClause::DropColumn { column_name } => write!(f, "DROP COLUMN `{column_name}`"),
            AlterTableClause::DropForeignKey { constraint_name } => write!(f, "DROP FOREIGN KEY `{constraint_name}`"),
            AlterTableClause::DropPrimaryKey => f.write_str("DROP PRIMARY KEY"),
            AlterTableClause::AddForeignKey(fk) => write!(f, "ADD {fk}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ForeignKey<'a> {
    pub constraint_name: Option<Cow<'a, str>>,
    pub constrained_columns: Vec<Cow<'a, str>>,
    pub referenced_table: Cow<'a, str>,
    pub referenced_columns: Vec<Cow<'a, str>>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

impl Display for ForeignKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(constraint_name) = &self.constraint_name {
            write!(f, "CONSTRAINT `{constraint_name}` ")?;
        }

        f.write_str("FOREIGN KEY (")?;

        self.constrained_columns.iter().map(|s| Ident(s)).join(", ", f)?;

        write!(f, ") REFERENCES `{}`(", self.referenced_table)?;

        self.referenced_columns.iter().map(|s| Ident(s)).join(", ", f)?;

        f.write_str(")")?;

        if let Some(on_delete) = &self.on_delete {
            f.write_str(" ON DELETE ")?;
            Display::fmt(on_delete, f)?;
        }

        if let Some(on_update) = &self.on_update {
            f.write_str(" ON UPDATE ")?;
            Display::fmt(&on_update, f)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ForeignKeyAction {
    Cascade,
    NoAction,
    Restrict,
    SetDefault,
    SetNull,
}

impl Display for ForeignKeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::NoAction => "NO ACTION",
            ForeignKeyAction::SetNull => "SET NULL",
            ForeignKeyAction::SetDefault => "SET DEFAULT",
        };

        f.write_str(s)
    }
}

#[derive(Debug, Default)]
pub struct Column<'a> {
    pub column_name: Cow<'a, str>,
    pub not_null: bool,
    pub column_type: Cow<'a, str>,
    pub default: Option<Cow<'a, str>>,
    pub auto_increment: bool,
    pub primary_key: bool,
    pub references: Option<ForeignKey<'a>>,
}

impl Display for Column<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&Ident(&self.column_name), f)?;
        f.write_str(" ")?;
        Display::fmt(&self.column_type, f)?;

        if self.not_null {
            f.write_str(" NOT NULL")?;
        } else {
            f.write_str(" NULL")?;
        }

        if self.auto_increment {
            f.write_str(" AUTO_INCREMENT")?;
        }

        if self.primary_key {
            f.write_str(" PRIMARY KEY")?;
        }

        if let Some(default) = &self.default {
            f.write_str(" DEFAULT ")?;
            f.write_str(default.as_ref())?;
        }

        if let Some(references) = &self.references {
            f.write_str(" ")?;
            Display::fmt(references, f)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CreateIndex<'a> {
    pub r#type: IndexType,
    pub index_name: Cow<'a, str>,
    pub on: (Cow<'a, str>, Vec<IndexColumn<'a>>),
}

impl Display for CreateIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CREATE ")?;

        match self.r#type {
            IndexType::Normal => (),
            IndexType::Unique => f.write_str("UNIQUE ")?,
            IndexType::Fulltext => f.write_str("FULLTEXT ")?,
        }

        f.write_str("INDEX `")?;
        f.write_str(&self.index_name)?;
        f.write_str("` ON `")?;
        f.write_str(&self.on.0)?;
        f.write_str("`(")?;

        self.on
            .1
            .iter()
            .map(|s| {
                let mut rendered = Ident(&s.name).to_string();

                if let Some(length) = s.length {
                    write!(rendered, "({length})").unwrap();
                }

                if let Some(sort_order) = s.sort_order {
                    rendered.push(' ');
                    rendered.push_str(sort_order.as_ref());
                }

                rendered
            })
            .join(", ", f)?;

        write!(f, ")")
    }
}

pub struct CreateTable<'a> {
    pub table_name: &'a dyn Display,
    pub columns: Vec<Column<'a>>,
    pub indexes: Vec<IndexClause<'a>>,
    pub primary_key: Vec<IndexColumn<'a>>,
    pub default_character_set: Option<Cow<'a, str>>,
    pub collate: Option<Cow<'a, str>>,
}

impl Display for CreateTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CREATE TABLE ")?;
        Display::fmt(self.table_name, f)?;

        f.write_str(" (\n")?;

        self.columns.iter().map(Indented).join(",\n", f)?;

        if !self.indexes.is_empty() || !self.primary_key.is_empty() {
            f.write_str(",\n\n")?;
        }

        self.indexes.iter().map(Indented).join(",\n", f)?;

        if !self.primary_key.is_empty() {
            if !self.indexes.is_empty() {
                f.write_str(",\n")?;
            }
            f.write_str(SQL_INDENTATION)?;
            f.write_str("PRIMARY KEY (")?;
            self.primary_key
                .iter()
                .map(|col| {
                    let mut rendered = Ident(&col.name).to_string();

                    if let Some(length) = col.length {
                        write!(rendered, "({length})").unwrap();
                    }

                    if let Some(sort_order) = col.sort_order {
                        rendered.push(' ');
                        rendered.push_str(sort_order.as_ref());
                    }

                    rendered
                })
                .join(", ", f)?;
            f.write_str(")")?;
        }

        f.write_str("\n)")?;

        if let Some(default_character_set) = &self.default_character_set {
            f.write_str(" DEFAULT CHARACTER SET ")?;
            f.write_str(default_character_set.as_ref())?;
        }

        if let Some(collate) = &self.collate {
            f.write_str(" COLLATE ")?;
            f.write_str(collate.as_ref())?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct DropTable<'a> {
    pub table_name: Cow<'a, str>,
}

impl Display for DropTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DROP TABLE `{}`", self.table_name)
    }
}

#[derive(Debug)]
pub struct DropIndex<'a> {
    pub index_name: Cow<'a, str>,
    pub table_name: Cow<'a, str>,
}

impl Display for DropIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DROP INDEX `{}` ON `{}`", self.index_name, self.table_name)
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub enum IndexType {
    #[default]
    Normal,
    Unique,
    Fulltext,
}

#[derive(Debug)]
pub struct IndexClause<'a> {
    pub index_name: Option<Cow<'a, str>>,
    pub r#type: IndexType,
    pub columns: Vec<IndexColumn<'a>>,
}

impl Display for IndexClause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.r#type {
            IndexType::Normal => (),
            IndexType::Unique => f.write_str("UNIQUE ")?,
            IndexType::Fulltext => f.write_str("FULLTEXT ")?,
        }

        f.write_str("INDEX ")?;

        if let Some(index_name) = &self.index_name {
            Display::fmt(&Ident(index_name.as_ref()), f)?;
        }

        f.write_str("(")?;

        self.columns
            .iter()
            .map(|col| {
                let mut rendered = format!("{}", Ident(col.name.as_ref()));

                if let Some(length) = col.length {
                    write!(rendered, "({length})").unwrap();
                };

                if let Some(sort_order) = col.sort_order {
                    rendered.push(' ');
                    rendered.push_str(sort_order.as_ref());
                };

                rendered
            })
            .join(", ", f)?;

        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn alter_table_add_foreign_key() {
        let alter_table = AlterTable {
            table_name: "Cat".into(),
            changes: vec![AlterTableClause::AddForeignKey(ForeignKey {
                constrained_columns: vec!["bestFriendId".into()],
                constraint_name: Some("myfk".into()),
                on_delete: Some(ForeignKeyAction::NoAction),
                on_update: Some(ForeignKeyAction::SetNull),
                referenced_columns: vec!["id".into()],
                referenced_table: "Dog".into(),
            })],
        };

        let expected = "ALTER TABLE `Cat` ADD CONSTRAINT `myfk` FOREIGN KEY (`bestFriendId`) REFERENCES `Dog`(`id`) ON DELETE NO ACTION ON UPDATE SET NULL";

        assert_eq!(alter_table.to_string(), expected);
    }

    #[test]
    fn full_create_table() {
        let stmt = CreateTable {
            table_name: &Ident("Cat"),
            columns: vec![
                Column {
                    column_type: "INTEGER".into(),
                    column_name: "id".into(),
                    not_null: false,
                    default: None,
                    auto_increment: true,
                    primary_key: true,
                    references: None,
                },
                Column {
                    column_type: "BINARY(16)".into(),
                    column_name: "test".into(),
                    not_null: true,
                    default: Some("(uuid_to_bin(uuid()))".into()),
                    auto_increment: false,
                    primary_key: false,
                    references: None,
                },
            ],
            indexes: vec![],
            default_character_set: Some("utf8mb4".into()),
            collate: Some("utf8mb4_unicode_ci".into()),
            primary_key: Vec::new(),
        };

        let expected = indoc!(
            r#"
            CREATE TABLE `Cat` (
                `id` INTEGER NULL AUTO_INCREMENT PRIMARY KEY,
                `test` BINARY(16) NOT NULL DEFAULT (uuid_to_bin(uuid()))
            ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci
            "#,
        )
        .trim_end();

        assert_eq!(stmt.to_string(), expected);
    }
}
