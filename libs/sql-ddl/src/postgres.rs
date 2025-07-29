use crate::common::{IndexColumn, IteratorJoin};
use std::{borrow::Cow, fmt::Display};

pub struct AlterTable<'a> {
    pub table_name: &'a dyn Display,
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

pub enum AlterTableClause<'a> {
    AddColumn(Column<'a>),
    AddForeignKey(ForeignKey<'a>),
    AddPrimaryKey(Vec<Cow<'a, str>>),
    DropColumn(Cow<'a, str>),
    DropConstraint(Cow<'a, str>),
    RenameTo(Cow<'a, str>),
}

impl Display for AlterTableClause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            AlterTableClause::AddColumn(col) => {
                f.write_str("ADD COLUMN ")?;
                Display::fmt(col, f)
            }
            AlterTableClause::AddForeignKey(fk) => {
                f.write_str("ADD ")?;
                Display::fmt(fk, f)
            }
            AlterTableClause::AddPrimaryKey(cols) => {
                f.write_str("ADD PRIMARY KEY (")?;

                cols.iter()
                    .map(|s| PostgresIdentifier::from(s.as_ref()))
                    .join(", ", f)?;

                f.write_str(")")
            }
            AlterTableClause::DropColumn(colname) => {
                f.write_str("DROP COLUMN ")?;
                Display::fmt(&PostgresIdentifier::from(colname.as_ref()), f)
            }
            AlterTableClause::DropConstraint(constraint_name) => {
                f.write_str("DROP CONSTRAINT ")?;
                Display::fmt(&PostgresIdentifier::from(constraint_name.as_ref()), f)
            }
            AlterTableClause::RenameTo(to) => {
                f.write_str("RENAME TO ")?;
                Display::fmt(&PostgresIdentifier::from(to.as_ref()), f)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub r#type: Cow<'a, str>,
    pub default: Option<Cow<'a, str>>,
}

impl Display for Column<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&PostgresIdentifier::from(self.name.as_ref()), f)?;
        f.write_str(" ")?;
        f.write_str(self.r#type.as_ref())?;

        if let Some(default) = &self.default {
            f.write_str(" DEFAULT ")?;
            f.write_str(default)?;
        }

        Ok(())
    }
}

/// Render a `DROP INDEX` statement.
///
/// ```
/// # use sql_ddl::postgres::DropIndex;
///
/// let drop_index = DropIndex { index_name: "Catidx".into() };
/// assert_eq!(drop_index.to_string(), r#"DROP INDEX "Catidx""#);
/// ```
#[derive(Debug)]
pub struct DropIndex<'a> {
    /// The name of the index to be dropped.
    pub index_name: PostgresIdentifier<'a>,
}

impl Display for DropIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DROP INDEX ")?;
        write!(f, "{}", self.index_name)
    }
}

/// Render a `DROP TABLE` statement.
///
/// ```
/// # use sql_ddl::postgres::DropTable;
///
/// let drop_table = DropTable { table_name: "Cat".into(), cascade: false };
/// assert_eq!(drop_table.to_string(), r#"DROP TABLE "Cat""#);
///
/// let drop_table = DropTable { table_name: "Cat".into(), cascade: true };
/// assert_eq!(drop_table.to_string(), r#"DROP TABLE "Cat" CASCADE"#);
/// ```
#[derive(Debug)]
pub struct DropTable<'a> {
    /// The name of the table to be dropped.
    pub table_name: PostgresIdentifier<'a>,
    pub cascade: bool,
}

impl Display for DropTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DROP TABLE ")?;
        Display::fmt(&self.table_name, f)?;

        if self.cascade {
            f.write_str(" CASCADE")?;
        }

        Ok(())
    }
}

/// Render a `DROP TYPE` statement.
///
/// ```
/// # use sql_ddl::postgres::DropType;
///
/// let drop_type = DropType { type_name: "CatMood".into() };
/// assert_eq!(drop_type.to_string(), r#"DROP TYPE "CatMood""#);
/// ```
#[derive(Debug)]
pub struct DropType<'a> {
    /// The name of the type to be dropped.
    pub type_name: PostgresIdentifier<'a>,
}

impl Display for DropType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DROP TYPE ")?;
        Display::fmt(&self.type_name, f)
    }
}

/// Render a `DROP VIEW` statement.
///
/// ```
/// # use sql_ddl::postgres::DropView;
///
/// let drop_view = DropView { view_name: "Cat".into() };
/// assert_eq!(drop_view.to_string(), r#"DROP VIEW "Cat""#);
/// ```
#[derive(Debug)]
pub struct DropView<'a> {
    /// The name of the view to be dropped.
    pub view_name: PostgresIdentifier<'a>,
}

impl Display for DropView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DROP VIEW ")?;
        Display::fmt(&self.view_name, f)
    }
}

pub struct ForeignKey<'a> {
    pub constraint_name: Option<Cow<'a, str>>,
    pub constrained_columns: Vec<Cow<'a, str>>,
    pub referenced_table: &'a dyn Display,
    pub referenced_columns: Vec<Cow<'a, str>>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

impl Display for ForeignKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(constraint_name) = &self.constraint_name {
            write!(f, "CONSTRAINT \"{constraint_name}\" ",)?;
        }

        f.write_str("FOREIGN KEY (")?;

        self.constrained_columns.iter().map(|s| Ident(s)).join(", ", f)?;

        write!(f, ") REFERENCES {}(", self.referenced_table)?;

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

#[derive(Debug)]
pub enum PostgresIdentifier<'a> {
    /// Simple identifier without a schema(namespace).
    Simple(Cow<'a, str>),
    /// Identifier with a schema(namespace). The first field is the schema.
    WithSchema(Cow<'a, str>, Cow<'a, str>),
}

impl<'a> PostgresIdentifier<'a> {
    pub fn new(namespace: Option<&'a str>, name: &'a str) -> Self {
        match namespace {
            Some(ns) => PostgresIdentifier::WithSchema(ns.into(), name.into()),
            None => PostgresIdentifier::Simple(name.into()),
        }
    }
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

impl Display for CreateEnum<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE TYPE {enum_name} AS ENUM (", enum_name = self.enum_name)?;
        self.variants.iter().map(|s| StrLit(s)).join(", ", f)?;
        f.write_str(")")
    }
}

pub enum IndexAlgorithm {
    BTree,
    Hash,
    Gist,
    Gin,
    SpGist,
    Brin,
}

pub struct CreateIndex<'a> {
    pub index_name: PostgresIdentifier<'a>,
    pub is_unique: bool,
    pub table_reference: &'a dyn Display,
    pub columns: Vec<IndexColumn<'a>>,
    pub using: Option<IndexAlgorithm>,
    pub where_clause: Option<&'a str>,
}

impl Display for CreateIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let using = match self.using {
            Some(IndexAlgorithm::Hash) => " USING HASH ",
            Some(IndexAlgorithm::Gist) => " USING GIST ",
            Some(IndexAlgorithm::Gin) => " USING GIN ",
            Some(IndexAlgorithm::SpGist) => " USING SPGIST ",
            Some(IndexAlgorithm::Brin) => " USING BRIN ",
            _ => "",
        };

        write!(
            f,
            "CREATE {uniqueness}INDEX {index_name} ON {table_reference}{using}(",
            uniqueness = if self.is_unique { "UNIQUE " } else { "" },
            index_name = self.index_name,
            table_reference = self.table_reference,
            using = using,
        )?;

        self.columns
            .iter()
            .map(|c| {
                let mut rendered = Ident(&c.name).to_string();

                if let Some(opclass) = &c.operator_class {
                    rendered.push(' ');
                    rendered.push_str(opclass.as_ref());
                }

                if let Some(sort_order) = c.sort_order {
                    rendered.push(' ');
                    rendered.push_str(sort_order.as_ref());
                }

                rendered
            })
            .join(", ", f)?;

        f.write_str(")")?;

        if let Some(where_clause) = self.where_clause {
            write!(f, " WHERE {}", where_clause)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::SortOrder;

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
        let columns = vec![IndexColumn::new("name"), IndexColumn::new("age")];

        let create_index = CreateIndex {
            is_unique: true,
            index_name: "meow_idx".into(),
            table_reference: &PostgresIdentifier::Simple(Cow::Borrowed("Cat")),
            columns,
            using: None,
        };

        assert_eq!(
            create_index.to_string(),
            "CREATE UNIQUE INDEX \"meow_idx\" ON \"Cat\"(\"name\", \"age\")"
        )
    }

    #[test]
    fn create_hash_index() {
        let columns = vec![IndexColumn::new("name")];

        let create_index = CreateIndex {
            is_unique: false,
            index_name: "meow_idx".into(),
            table_reference: &PostgresIdentifier::Simple(Cow::Borrowed("Cat")),
            columns,
            using: Some(IndexAlgorithm::Hash),
        };

        assert_eq!(
            create_index.to_string(),
            "CREATE INDEX \"meow_idx\" ON \"Cat\" USING HASH (\"name\")"
        )
    }

    #[test]
    fn create_unique_index_sort_order() {
        let columns = vec![
            IndexColumn {
                name: "name".into(),
                sort_order: Some(SortOrder::Asc),
                ..Default::default()
            },
            IndexColumn {
                name: "age".into(),
                sort_order: Some(SortOrder::Desc),
                ..Default::default()
            },
        ];

        let create_index = CreateIndex {
            is_unique: true,
            index_name: "meow_idx".into(),
            table_reference: &PostgresIdentifier::Simple("Cat".into()),
            columns,
            using: None,
        };

        assert_eq!(
            create_index.to_string(),
            "CREATE UNIQUE INDEX \"meow_idx\" ON \"Cat\"(\"name\" ASC, \"age\" DESC)"
        )
    }

    #[test]
    fn full_alter_table_add_foreign_key() {
        let alter_table = AlterTable {
            table_name: &PostgresIdentifier::WithSchema("public".into(), "Cat".into()),
            clauses: vec![AlterTableClause::AddForeignKey(ForeignKey {
                constrained_columns: vec!["friendName".into(), "friendTemperament".into()],
                constraint_name: Some("cat_friend".into()),
                on_delete: None,
                on_update: None,
                referenced_columns: vec!["name".into(), "temperament".into()],
                referenced_table: &"Dog",
            })],
        };

        let expected = "ALTER TABLE \"public\".\"Cat\" ADD CONSTRAINT \"cat_friend\" FOREIGN KEY (\"friendName\", \"friendTemperament\") REFERENCES Dog(\"name\", \"temperament\")";

        assert_eq!(alter_table.to_string(), expected);
    }

    #[test]
    fn rename_table() {
        let expected = r#"ALTER TABLE Cat RENAME TO "Dog""#;
        let alter_table = AlterTable {
            table_name: &"Cat",
            clauses: vec![AlterTableClause::RenameTo("Dog".into())],
        };

        assert_eq!(alter_table.to_string(), expected);
    }
}
