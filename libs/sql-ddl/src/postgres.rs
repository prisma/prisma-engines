use std::{borrow::Cow, fmt::Display};

pub enum PostgresIdentifier<'a> {
    Simple(Cow<'a, str>),
    WithSchema(Cow<'a, str>, Cow<'a, str>),
}

impl<'a> From<&'a str> for PostgresIdentifier<'a> {
    fn from(s: &'a str) -> Self {
        PostgresIdentifier::Simple(Cow::Borrowed(s))
    }
}

impl<'a> From<(&'a str, &'a str)> for PostgresIdentifier<'a> {
    fn from((schema, item): (&'a str, &'a str)) -> Self {
        PostgresIdentifier::WithSchema(Cow::Borrowed(schema), Cow::Borrowed(item))
    }
}

impl<'a> Display for PostgresIdentifier<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostgresIdentifier::Simple(ident) => write!(f, "\"{}\"", ident),
            PostgresIdentifier::WithSchema(schema_name, ident) => write!(f, "\"{}\".\"{}\"", schema_name, ident),
        }
    }
}

pub struct CreateEnum<'a> {
    pub enum_name: PostgresIdentifier<'a>,
    pub variants: Vec<Cow<'a, str>>,
}

impl<'a> Display for CreateEnum<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE TYPE {enum_name} AS ENUM (", enum_name = self.enum_name)?;

        let mut variants = self.variants.iter().peekable();

        while let Some(variant) = variants.next() {
            write!(f, "'{variant}'", variant = variant)?;

            if variants.peek().is_some() {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")
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

        let mut columns = self.columns.iter().peekable();

        while let Some(column) = columns.next() {
            write!(f, "\"{}\"", column)?;

            if columns.peek().is_some() {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")
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
}
