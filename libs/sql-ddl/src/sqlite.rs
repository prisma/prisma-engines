use crate::common::{Indented, IteratorJoin, SQL_INDENTATION};
use std::{borrow::Cow, fmt::Display};

struct SqliteIdentifier<T>(T);

impl<T: Display> Display for SqliteIdentifier<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

pub struct CreateTable<'a> {
    pub table_name: &'a dyn Display,
    pub columns: Vec<Column<'a>>,
    pub primary_key: Option<Vec<Cow<'a, str>>>,
    pub foreign_keys: Vec<ForeignKey<'a>>,
}

impl Display for CreateTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "CREATE TABLE {} (", self.table_name)?;

        self.columns.iter().map(Indented).join(",\n", f)?;

        if let Some(primary_key) = &self.primary_key {
            f.write_str(",\n\n")?;
            f.write_str(SQL_INDENTATION)?;
            f.write_str("PRIMARY KEY (")?;
            primary_key.iter().map(SqliteIdentifier).join(", ", f)?;
            f.write_str(")")?;
        }

        for foreign_key in &self.foreign_keys {
            write!(f, ",\n{SQL_INDENTATION}{foreign_key}")?;
        }

        write!(f, "\n)")
    }
}

#[derive(Debug, Default)]
pub struct ForeignKey<'a> {
    pub constrains: Vec<Cow<'a, str>>,
    pub references: (Cow<'a, str>, Vec<Cow<'a, str>>),
    pub constraint_name: Option<Cow<'a, str>>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

/// Foreign key action types (for ON DELETE|ON UPDATE).
#[derive(Debug)]
pub enum ForeignKeyAction {
    /// Produce an error indicating that the deletion or update would create a foreign key
    /// constraint violation. If the constraint is deferred, this error will be produced at
    /// constraint check time if there still exist any referencing rows. This is the default action.
    NoAction,
    /// Produce an error indicating that the deletion or update would create a foreign key
    /// constraint violation. This is the same as NO ACTION except that the check is not deferrable.
    Restrict,
    /// Delete any rows referencing the deleted row, or update the values of the referencing
    /// column(s) to the new values of the referenced columns, respectively.
    Cascade,
    /// Set the referencing column(s) to null.
    SetNull,
    /// Set the referencing column(s) to their default values. (There must be a row in the
    /// referenced table matching the default values, if they are not null, or the operation
    /// will fail).
    SetDefault,
}

impl Display for ForeignKeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let action_s = match self {
            ForeignKeyAction::NoAction => "NO ACTION",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::SetNull => "SET NULL",
            ForeignKeyAction::SetDefault => "SET DEFAULT",
        };

        f.write_str(action_s)
    }
}

impl Display for ForeignKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(constraint_name) = &self.constraint_name {
            write!(f, "CONSTRAINT \"{constraint_name}\" ")?;
        }

        f.write_str("FOREIGN KEY (")?;

        self.constrains.iter().map(SqliteIdentifier).join(", ", f)?;

        write!(
            f,
            ") REFERENCES \"{referenced_table}\" (",
            referenced_table = self.references.0,
        )?;

        self.references.1.iter().map(SqliteIdentifier).join(", ", f)?;

        f.write_str(")")?;

        if let Some(action) = &self.on_delete {
            f.write_str(" ON DELETE ")?;
            action.fmt(f)?;
        }

        if let Some(action) = &self.on_update {
            f.write_str(" ON UPDATE ")?;
            action.fmt(f)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub r#type: Cow<'a, str>,
    pub not_null: bool,
    pub primary_key: bool,
    pub default: Option<Cow<'a, str>>,
    /// Whether to render AUTOINCREMENT on the primary key.
    pub autoincrement: bool,
}

impl Display for Column<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"{name}\" {tpe}{not_null}{primary_key}{autoincrement}",
            name = self.name,
            tpe = self.r#type,
            not_null = if self.not_null { " NOT NULL" } else { "" },
            primary_key = if self.primary_key { " PRIMARY KEY" } else { "" },
            autoincrement = if self.autoincrement { " AUTOINCREMENT" } else { "" },
        )?;

        if let Some(default) = &self.default {
            f.write_str(" DEFAULT ")?;
            f.write_str(default)?;
        }

        Ok(())
    }
}

/// A column in an index definition.
#[derive(Debug, Default)]
pub struct IndexColumn<'a> {
    pub name: Cow<'a, str>,
    pub sort_order: Option<crate::SortOrder>,
}

/// Create an index statement.
#[derive(Debug)]
pub struct CreateIndex<'a> {
    pub index_name: Cow<'a, str>,
    pub table_name: Cow<'a, str>,
    pub columns: Vec<IndexColumn<'a>>,
    pub is_unique: bool,
    pub where_clause: Option<Cow<'a, str>>,
}

impl Display for CreateIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CREATE {uniqueness}INDEX \"{index_name}\" ON \"{table_name}\"(",
            uniqueness = if self.is_unique { "UNIQUE " } else { "" },
            index_name = self.index_name,
            table_name = self.table_name,
        )?;

        for (i, c) in self.columns.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "\"{}\"", c.name)?;
            if let Some(sort_order) = c.sort_order {
                write!(f, " {}", sort_order.as_ref())?;
            }
        }

        f.write_str(")")?;

        if let Some(predicate) = &self.where_clause {
            write!(f, " WHERE {}", predicate)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn basic_create_table() {
        let create_table = CreateTable {
            table_name: &SqliteIdentifier("Cat"),
            columns: vec![
                Column {
                    name: "id".into(),
                    r#type: "integer".into(),
                    primary_key: true,
                    autoincrement: true,
                    ..Default::default()
                },
                Column {
                    name: "boxId".into(),
                    r#type: "uuid".into(),
                    ..Default::default()
                },
            ],
            primary_key: None,
            foreign_keys: Vec::new(),
        };

        let expected = indoc::indoc!(
            r#"
            CREATE TABLE "Cat" (
                "id" integer PRIMARY KEY AUTOINCREMENT,
                "boxId" uuid
            )
            "#
        );

        assert_eq!(create_table.to_string(), expected.trim_matches('\n'))
    }

    #[test]
    fn create_table_with_primary_key() {
        let create_table = CreateTable {
            table_name: &SqliteIdentifier("Cat"),
            columns: vec![
                Column {
                    name: "id".into(),
                    r#type: "integer".into(),
                    ..Default::default()
                },
                Column {
                    name: "boxId".into(),
                    r#type: "uuid".into(),
                    default: Some("'maybe_a_uuid_idk'".into()),
                    ..Default::default()
                },
            ],
            primary_key: Some(vec!["id".into(), "boxId".into()]),
            foreign_keys: Vec::new(),
        };

        let expected = indoc!(
            r#"
            CREATE TABLE "Cat" (
                "id" integer,
                "boxId" uuid DEFAULT 'maybe_a_uuid_idk',

                PRIMARY KEY ("id", "boxId")
            )
            "#
        );

        assert_eq!(create_table.to_string(), expected.trim_matches('\n'))
    }

    #[test]
    fn create_table_with_primary_key_and_foreign_keys() {
        let create_table = CreateTable {
            table_name: &SqliteIdentifier("Cat"),
            columns: vec![
                Column {
                    name: "id".into(),
                    r#type: "integer".into(),
                    ..Default::default()
                },
                Column {
                    name: "boxId".into(),
                    r#type: "uuid".into(),
                    default: Some("'maybe_a_uuid_idk'".into()),
                    ..Default::default()
                },
            ],
            primary_key: Some(vec!["id".into(), "boxId".into()]),
            foreign_keys: vec![
                ForeignKey {
                    constrains: vec!["boxId".into()],
                    references: ("Box".into(), vec!["id".into(), "material".into()]),
                    ..Default::default()
                },
                ForeignKey {
                    constrains: vec!["id".into()],
                    references: ("meow".into(), vec!["id".into()]),
                    constraint_name: Some("meowConstraint".into()),
                    ..Default::default()
                },
            ],
        };

        let expected = indoc!(
            r#"
            CREATE TABLE "Cat" (
                "id" integer,
                "boxId" uuid DEFAULT 'maybe_a_uuid_idk',

                PRIMARY KEY ("id", "boxId"),
                FOREIGN KEY ("boxId") REFERENCES "Box" ("id", "material"),
                CONSTRAINT "meowConstraint" FOREIGN KEY ("id") REFERENCES "meow" ("id")
            )
            "#
        );

        assert_eq!(create_table.to_string(), expected.trim_matches('\n'))
    }

    #[test]
    fn create_unique_index() {
        let create_index = CreateIndex {
            index_name: "idx_name".into(),
            table_name: "Cat".into(),
            columns: vec![IndexColumn {
                name: "name".into(),
                sort_order: None,
            }],
            is_unique: true,
            where_clause: None,
        };

        assert_eq!(
            create_index.to_string(),
            r#"CREATE UNIQUE INDEX "idx_name" ON "Cat"("name")"#
        )
    }

    #[test]
    fn create_partial_unique_index() {
        let create_index = CreateIndex {
            index_name: "idx_name".into(),
            table_name: "Cat".into(),
            columns: vec![IndexColumn {
                name: "name".into(),
                sort_order: None,
            }],
            is_unique: true,
            where_clause: Some("status = 'active'".into()),
        };

        assert_eq!(
            create_index.to_string(),
            r#"CREATE UNIQUE INDEX "idx_name" ON "Cat"("name") WHERE status = 'active'"#
        )
    }
}
