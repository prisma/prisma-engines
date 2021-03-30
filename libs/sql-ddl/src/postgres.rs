use crate::common::IteratorJoin;
use std::{borrow::Cow, fmt::Display, todo};

#[derive(Debug, Default)]
pub struct AlterTable<'a> {
    pub table_name: PostgresIdentifier<'a>,
    pub clauses: Vec<AlterTableClause<'a>>,
}

impl Display for AlterTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ALTER TABLE ")?;
        self.table_name.fmt(f)?;

        if self.clauses.len() <= 1 {
            f.write_str(" ")?;
            self.clauses[0].fmt(f)?;
        } else {
            todo!("multiline ALTER TABLE")
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum AlterTableClause<'a> {
    AddForeignKey(ForeignKey<'a>),
    RenameTo(Cow<'a, str>),
}

impl Display for AlterTableClause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            AlterTableClause::AddForeignKey(fk) => {
                f.write_str("ADD ")?;
                Display::fmt(fk, f)
            }
            &AlterTableClause::RenameTo(to) => {
                f.write_str("RENAME TO ")?;
                Display::fmt(&PostgresIdentifier::from(to.as_ref()), f)
            }
        }
    }
}

#[derive(Debug)]
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
            write!(
                f,
                "CONSTRAINT \"{constraint_name}\" ",
                constraint_name = constraint_name,
            )?;
        }

        f.write_str("FOREIGN KEY (")?;

        self.constrained_columns.iter().map(|s| Ident(s)).join(", ", f)?;

        write!(f, ") REFERENCES \"{}\"(", self.referenced_table)?;

        self.referenced_columns.iter().map(|s| Ident(s)).join(", ", f)?;

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
pub enum PostgresIdentifier<'a> {
    Simple(Cow<'a, str>),
    WithSchema(Cow<'a, str>, Cow<'a, str>),
}

impl Default for PostgresIdentifier<'_> {
    fn default() -> Self {
        PostgresIdentifier::Simple(Cow::Borrowed(""))
    }
}

impl<'a> From<&'a str> for PostgresIdentifier<'a> {
    fn from(s: &'a str) -> Self {
        PostgresIdentifier::Simple(Cow::Borrowed(s))
    }
}

impl Display for PostgresIdentifier<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let delimiter = "\"";

        match self {
            PostgresIdentifier::Simple(name) => {
                f.write_str(delimiter)?;
                f.write_str(name)?;
                f.write_str(delimiter)
            }
            PostgresIdentifier::WithSchema(prefix, name) => {
                f.write_str(delimiter)?;
                f.write_str(prefix)?;
                f.write_str(r#"".""#)?;
                f.write_str(name)?;
                f.write_str(delimiter)
            }
        }
    }
}

struct StrLit<'a>(&'a str);

impl Display for StrLit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0)?;
        Ok(())
    }
}

struct Ident<'a>(&'a str);

impl Display for Ident<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)?;
        Ok(())
    }
}

impl<'a> From<(&'a str, &'a str)> for PostgresIdentifier<'a> {
    fn from((schema, item): (&'a str, &'a str)) -> Self {
        PostgresIdentifier::WithSchema(Cow::Borrowed(schema), Cow::Borrowed(item))
    }
}

pub struct CreateEnum<'a> {
    pub enum_name: PostgresIdentifier<'a>,
    pub variants: Vec<Cow<'a, str>>,
}

impl<'a> Display for CreateEnum<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE TYPE {enum_name} AS ENUM (", enum_name = self.enum_name)?;
        self.variants.iter().map(|s| StrLit(s)).join(", ", f)?;
        f.write_str(")")
    }
}

pub struct CreateIndex<'a> {
    pub index_name: PostgresIdentifier<'a>,
    pub is_unique: bool,
    pub table_reference: PostgresIdentifier<'a>,
    pub columns: Vec<Cow<'a, str>>,
}

impl<'a> Display for CreateIndex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CREATE {uniqueness}INDEX {index_name} ON {table_reference}(",
            uniqueness = if self.is_unique { "UNIQUE " } else { "" },
            index_name = self.index_name,
            table_reference = self.table_reference,
        )?;

        self.columns.iter().map(|s| Ident(s)).join(", ", f)?;

        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_enum_without_variants() {
        let create_enum = CreateEnum {
            enum_name: "myEnum".into(),
            variants: Vec::new(),
        };

        assert_eq!(create_enum.to_string(), r#"CREATE TYPE "myEnum" AS ENUM ()"#);
    }

    #[test]
    fn create_enum_with_variants() {
        let variants = vec!["One".into(), "Two".into(), "Three".into()];
        let create_enum = CreateEnum {
            enum_name: "myEnum".into(),
            variants,
        };

        assert_eq!(
            create_enum.to_string(),
            r#"CREATE TYPE "myEnum" AS ENUM ('One', 'Two', 'Three')"#
        );
    }

    #[test]
    fn create_unique_index() {
        let columns = vec!["name".into(), "age".into()];

        let create_index = CreateIndex {
            is_unique: true,
            index_name: "meow_idx".into(),
            table_reference: "Cat".into(),
            columns,
        };

        assert_eq!(
            create_index.to_string(),
            "CREATE UNIQUE INDEX \"meow_idx\" ON \"Cat\"(\"name\", \"age\")"
        )
    }

    #[test]
    fn full_alter_table_add_foreign_key() {
        let alter_table = AlterTable {
            table_name: PostgresIdentifier::WithSchema("public".into(), "Cat".into()),
            clauses: vec![AlterTableClause::AddForeignKey(ForeignKey {
                constrained_columns: vec!["friendName".into(), "friendTemperament".into()],
                constraint_name: Some("cat_friend".into()),
                on_delete: None,
                on_update: None,
                referenced_columns: vec!["name".into(), "temperament".into()],
                referenced_table: "Dog".into(),
            })],
        };

        let expected =
            "ALTER TABLE \"public\".\"Cat\" ADD CONSTRAINT \"cat_friend\" FOREIGN KEY (\"friendName\", \"friendTemperament\") REFERENCES \"Dog\"(\"name\", \"temperament\")";

        assert_eq!(alter_table.to_string(), expected);
    }

    #[test]
    fn rename_table() {
        let expected = r#"ALTER TABLE "Cat" RENAME TO "Dog""#;
        let alter_table = AlterTable {
            table_name: "Cat".into(),
            clauses: vec![AlterTableClause::RenameTo("Dog".into())],
            ..Default::default()
        };

        assert_eq!(alter_table.to_string(), expected);
    }
}
