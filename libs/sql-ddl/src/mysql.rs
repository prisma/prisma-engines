use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Default)]
pub struct AlterTable<'a> {
    pub table_name: Cow<'a, str>,
    pub changes: Vec<AlterTableClause<'a>>,
}

impl Display for AlterTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ALTER TABLE `{}`", self.table_name)?;

        if self.changes.len() == 1 {
            write!(f, " {}", self.changes[0])?;

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
            AlterTableClause::RenameTo { next_name } => write!(f, "RENAME TO {}", next_name),
            AlterTableClause::RenameIndex {
                previous_name,
                next_name,
            } => write!(f, "RENAME INDEX `{}` TO `{}`", previous_name, next_name),
            AlterTableClause::DropColumn { column_name } => write!(f, "DROP COLUMN `{}`", column_name),
            AlterTableClause::DropForeignKey { constraint_name } => write!(f, "DROP FOREIGN KEY `{}`", constraint_name),
            AlterTableClause::DropPrimaryKey => f.write_str("DROP PRIMARY KEY"),
            AlterTableClause::AddForeignKey(fk) => write!(f, "ADD {}", fk),
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

impl<'a> Display for ForeignKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(constraint_name) = &self.constraint_name {
            write!(f, "CONSTRAINT `{constraint_name}` ", constraint_name = constraint_name,)?;
        }

        f.write_str("FOREIGN KEY (")?;

        let mut constrained_columns = self.constrained_columns.iter().peekable();

        while let Some(column) = constrained_columns.next() {
            write!(f, "`{}`", column)?;

            if constrained_columns.peek().is_some() {
                f.write_str(", ")?;
            }
        }

        write!(f, ") REFERENCES `{}`(", self.referenced_table)?;

        let mut referenced_columns = self.referenced_columns.iter().peekable();

        while let Some(column) = referenced_columns.next() {
            write!(f, "`{}`", column)?;

            if referenced_columns.peek().is_some() {
                f.write_str(", ")?;
            }
        }

        f.write_str(")")?;

        if let Some(on_delete) = &self.on_delete {
            f.write_str(" ON DELETE ")?;
            on_delete.fmt(f)?;
        }

        if let Some(on_update) = &self.on_update {
            f.write_str(" ON UPDATE ")?;
            on_update.fmt(f)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ForeignKeyAction {
    Cascade,
    DoNothing,
    Restrict,
    SetDefault,
    SetNull,
}

impl Display for ForeignKeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::DoNothing => "DO NOTHING",
            ForeignKeyAction::SetNull => "SET NULL",
            ForeignKeyAction::SetDefault => "SET DEFAULT",
        };

        f.write_str(s)
    }
}

#[derive(Debug)]
pub struct CreateIndex<'a> {
    pub unique: bool,
    pub index_name: Cow<'a, str>,
    pub on: (Cow<'a, str>, Vec<Cow<'a, str>>),
}

impl Display for CreateIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CREATE {maybe_unique}INDEX `{index_name}` ON `{table_name}`(",
            maybe_unique = if self.unique { "UNIQUE " } else { "" },
            index_name = self.index_name,
            table_name = self.on.0,
        )?;

        let mut columns = self.on.1.iter().peekable();

        while let Some(column_name) = columns.next() {
            write!(f, "`{}`", column_name)?;

            if columns.peek().is_some() {
                f.write_str(", ")?;
            }
        }

        write!(f, ")")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alter_table_add_foreign_key() {
        let alter_table = AlterTable {
            table_name: "Cat".into(),
            changes: vec![AlterTableClause::AddForeignKey(ForeignKey {
                constrained_columns: vec!["bestFriendId".into()],
                constraint_name: Some("myfk".into()),
                on_delete: Some(ForeignKeyAction::DoNothing),
                on_update: Some(ForeignKeyAction::SetNull),
                referenced_columns: vec!["id".into()],
                referenced_table: "Dog".into(),
            })],
        };

        let expected = "ALTER TABLE `Cat` ADD CONSTRAINT `myfk` FOREIGN KEY (`bestFriendId`) REFERENCES `Dog`(`id`) ON DELETE DO NOTHING ON UPDATE SET NULL";

        assert_eq!(alter_table.to_string(), expected);
    }
}
