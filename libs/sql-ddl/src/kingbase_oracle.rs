use crate::common::{Indented, IndexColumn, IteratorJoin};
use std::{borrow::Cow, fmt::Display};

pub struct OracleIdentifier<'a>(pub &'a str);

impl Display for OracleIdentifier<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0.replace('"', "\"\""))
    }
}

pub struct OracleString<'a>(pub &'a str);

impl Display for OracleString<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0.replace('\'', "''"))
    }
}

pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub r#type: Cow<'a, str>,
    pub not_null: bool,
    pub default: Option<Cow<'a, str>>,
}

impl Display for Column<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", OracleIdentifier(&self.name), self.r#type)?;
        if self.not_null {
            f.write_str(" NOT NULL")?;
        }
        if let Some(default) = &self.default {
            write!(f, " DEFAULT {default}")?;
        }
        Ok(())
    }
}

pub struct PrimaryKey<'a> {
    pub constraint_name: Cow<'a, str>,
    pub columns: Vec<Cow<'a, str>>,
}

impl Display for PrimaryKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CONSTRAINT {} PRIMARY KEY (",
            OracleIdentifier(&self.constraint_name)
        )?;
        self.columns.iter().map(|name| OracleIdentifier(name)).join(", ", f)?;
        f.write_str(")")
    }
}

pub struct CreateTable<'a> {
    pub table_name: &'a dyn Display,
    pub columns: Vec<Column<'a>>,
    pub primary_key: Option<PrimaryKey<'a>>,
}

impl Display for CreateTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CREATE TABLE {} (", self.table_name)?;
        self.columns.iter().map(Indented).join(",\n", f)?;
        if let Some(primary_key) = &self.primary_key {
            if !self.columns.is_empty() {
                f.write_str(",\n")?;
            }
            Display::fmt(&Indented(primary_key), f)?;
        }
        f.write_str("\n)")
    }
}

pub struct CreateIndex<'a> {
    pub index_name: Cow<'a, str>,
    pub table_name: &'a dyn Display,
    pub columns: Vec<IndexColumn<'a>>,
    pub is_unique: bool,
}

impl Display for CreateIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CREATE ")?;
        if self.is_unique {
            f.write_str("UNIQUE ")?;
        }
        write!(
            f,
            "INDEX {} ON {} (",
            OracleIdentifier(&self.index_name),
            self.table_name
        )?;
        self.columns
            .iter()
            .map(|column| {
                let mut rendered = OracleIdentifier(&column.name).to_string();
                if let Some(sort_order) = column.sort_order {
                    rendered.push(' ');
                    rendered.push_str(sort_order.as_ref());
                }
                rendered
            })
            .join(", ", f)?;
        f.write_str(")")
    }
}

pub struct ForeignKey<'a> {
    pub constraint_name: Option<Cow<'a, str>>,
    pub constrained_columns: Vec<Cow<'a, str>>,
    pub referenced_table: &'a dyn Display,
    pub referenced_columns: Vec<Cow<'a, str>>,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
}

impl Display for ForeignKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.constraint_name {
            write!(f, "CONSTRAINT {} ", OracleIdentifier(name))?;
        }
        f.write_str("FOREIGN KEY (")?;
        self.constrained_columns
            .iter()
            .map(|name| OracleIdentifier(name))
            .join(", ", f)?;
        write!(f, ") REFERENCES {} (", self.referenced_table)?;
        self.referenced_columns
            .iter()
            .map(|name| OracleIdentifier(name))
            .join(", ", f)?;
        write!(f, ") ON DELETE {} ON UPDATE {}", self.on_delete, self.on_update)
    }
}

pub enum ForeignKeyAction {
    Cascade,
    NoAction,
    Restrict,
    SetDefault,
    SetNull,
}

impl Display for ForeignKeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Cascade => "CASCADE",
            Self::NoAction => "NO ACTION",
            Self::Restrict => "RESTRICT",
            Self::SetDefault => "SET DEFAULT",
            Self::SetNull => "SET NULL",
        })
    }
}

pub struct AlterTable<'a> {
    pub table_name: &'a dyn Display,
    pub clauses: Vec<AlterTableClause<'a>>,
}

impl Display for AlterTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ALTER TABLE {} ", self.table_name)?;
        self.clauses.iter().join(", ", f)
    }
}

pub enum AlterTableClause<'a> {
    AddForeignKey(ForeignKey<'a>),
    AddColumn(Column<'a>),
    AddUnique {
        constraint_name: Cow<'a, str>,
        columns: Vec<Cow<'a, str>>,
    },
    DropColumn(Cow<'a, str>),
    RenameTo(Cow<'a, str>),
}

impl Display for AlterTableClause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddForeignKey(foreign_key) => write!(f, "ADD {foreign_key}"),
            Self::AddColumn(column) => write!(f, "ADD ({column})"),
            Self::AddUnique {
                constraint_name,
                columns,
            } => {
                write!(f, "ADD CONSTRAINT {} UNIQUE (", OracleIdentifier(constraint_name))?;
                columns.iter().map(|name| OracleIdentifier(name)).join(", ", f)?;
                f.write_str(")")
            }
            Self::DropColumn(name) => write!(f, "DROP COLUMN {}", OracleIdentifier(name)),
            Self::RenameTo(name) => write!(f, "RENAME TO {}", OracleIdentifier(name)),
        }
    }
}

pub struct CreateEnum<'a> {
    pub enum_name: &'a dyn Display,
    pub variants: Vec<Cow<'a, str>>,
}

impl Display for CreateEnum<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE TYPE {} AS ENUM (", self.enum_name)?;
        self.variants.iter().map(|value| OracleString(value)).join(", ", f)?;
        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SortOrder;
    use indoc::indoc;

    #[test]
    fn create_table_with_quoted_names_defaults_and_a_composite_primary_key() {
        let table = CreateTable {
            table_name: &OracleIdentifier("a\"b"),
            columns: vec![
                Column {
                    name: "i\"d".into(),
                    r#type: "NUMBER(10,0)".into(),
                    not_null: true,
                    default: None,
                },
                Column {
                    name: "name".into(),
                    r#type: "VARCHAR2(64)".into(),
                    not_null: false,
                    default: Some(OracleString("O'Reilly\\path").to_string().into()),
                },
            ],
            primary_key: Some(PrimaryKey {
                constraint_name: "p\"k".into(),
                columns: vec!["i\"d".into(), "name".into()],
            }),
        };
        assert_eq!(
            table.to_string(),
            indoc! {r#"
            CREATE TABLE "a""b" (
                "i""d" NUMBER(10,0) NOT NULL,
                "name" VARCHAR2(64) DEFAULT 'O''Reilly\path',
                CONSTRAINT "p""k" PRIMARY KEY ("i""d", "name")
            )"#}
        );
    }

    #[test]
    fn create_unique_index_with_sort_order() {
        let index = CreateIndex {
            index_name: "i\"x".into(),
            table_name: &OracleIdentifier("items"),
            columns: vec![IndexColumn {
                name: "value".into(),
                sort_order: Some(SortOrder::Desc),
                ..Default::default()
            }],
            is_unique: true,
        };
        assert_eq!(
            index.to_string(),
            "CREATE UNIQUE INDEX \"i\"\"x\" ON \"items\" (\"value\" DESC)"
        );
    }

    #[test]
    fn alter_table_add_foreign_key_preserves_both_referential_actions() {
        let table = OracleIdentifier("Child");
        let parent = OracleIdentifier("Parent");
        let alter = AlterTable {
            table_name: &table,
            clauses: vec![AlterTableClause::AddForeignKey(ForeignKey {
                constraint_name: Some("f\"k".into()),
                constrained_columns: vec!["parentId".into()],
                referenced_table: &parent,
                referenced_columns: vec!["id".into()],
                on_delete: ForeignKeyAction::SetDefault,
                on_update: ForeignKeyAction::Cascade,
            })],
        };
        assert_eq!(
            alter.to_string(),
            "ALTER TABLE \"Child\" ADD CONSTRAINT \"f\"\"k\" FOREIGN KEY (\"parentId\") REFERENCES \"Parent\" (\"id\") ON DELETE SET DEFAULT ON UPDATE CASCADE"
        );
    }

    #[test]
    fn create_enum_preserves_literal_quotes_and_backslashes() {
        let enm = CreateEnum {
            enum_name: &OracleIdentifier("State"),
            variants: vec!["O'Reilly".into(), "a\\b".into()],
        };
        assert_eq!(enm.to_string(), "CREATE TYPE \"State\" AS ENUM ('O''Reilly', 'a\\b')");
    }
}
