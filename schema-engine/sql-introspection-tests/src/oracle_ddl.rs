//! Oracle-mode DDL for the portable Barrel fixtures. This is a test backend,
//! independent of the connector's renderer so introspection tests do not use
//! the implementation under test to build their input schema.
use barrel::{
    DatabaseChange, Migration,
    backend::SqlGenerator,
    functions::AutogenFunction,
    types::{BaseType, Constraint, Type, WrappedDefault},
};
use std::cell::RefCell;

#[derive(Default)]
struct Sequences {
    table: String,
    before: String,
    after: String,
}

thread_local! {
    // Barrel's SqlGenerator methods are static and add_column has no table
    // argument. render() supplies that context synchronously, once per table.
    static SEQUENCES: RefCell<Sequences> = RefCell::default();
}

fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn qualified(schema: Option<&str>, name: &str) -> String {
    match schema {
        Some(schema) => format!("{}.{}", ident(schema), ident(name)),
        None => ident(name),
    }
}

fn columns(names: &[String]) -> String {
    names.iter().map(|name| ident(name)).collect::<Vec<_>>().join(", ")
}

pub(crate) fn render(migration: &Migration) -> String {
    let mut sql = String::new();
    for change in &migration.changes {
        let table = match change {
            DatabaseChange::CreateTable(table, _)
            | DatabaseChange::CreateTableIfNotExists(table, _)
            | DatabaseChange::ChangeTable(table, _) => table.meta.name(),
            _ => String::new(),
        };
        SEQUENCES.with_borrow_mut(|state| {
            *state = Sequences {
                table,
                ..Default::default()
            }
        });
        let statement = Migration {
            schema: migration.schema.clone(),
            changes: vec![change.clone()],
        }
        .make::<Oracle>();
        SEQUENCES.with_borrow_mut(|state| {
            sql.push_str(&state.before);
            sql.push_str(&statement);
            sql.push_str(&state.after);
            *state = Sequences::default();
        });
    }
    sql
}

struct Oracle;

impl SqlGenerator for Oracle {
    fn create_table(name: &str, schema: Option<&str>) -> String {
        format!("CREATE TABLE {}", qualified(schema, name))
    }

    fn create_table_if_not_exists(name: &str, schema: Option<&str>) -> String {
        format!("CREATE TABLE IF NOT EXISTS {}", qualified(schema, name))
    }

    fn drop_table(name: &str, schema: Option<&str>) -> String {
        format!("DROP TABLE {}", qualified(schema, name))
    }

    fn drop_table_if_exists(name: &str, schema: Option<&str>) -> String {
        format!("DROP TABLE IF EXISTS {}", qualified(schema, name))
    }

    fn rename_table(old: &str, new: &str, schema: Option<&str>) -> String {
        format!("ALTER TABLE {} RENAME TO {}", qualified(schema, old), ident(new))
    }

    fn alter_table(name: &str, schema: Option<&str>) -> String {
        format!("ALTER TABLE {}", qualified(schema, name))
    }

    fn add_column(existing: bool, schema: Option<&str>, name: &str, tpe: &Type) -> String {
        let mut sql = format!(
            "{}{} {}",
            if existing { "ADD COLUMN " } else { "" },
            ident(name),
            match &tpe.inner {
                BaseType::Primary | BaseType::Serial | BaseType::Integer => "NUMBER(10,0)".into(),
                BaseType::Text => "CLOB".into(),
                BaseType::Varchar(0) => "VARCHAR2(4000)".into(),
                BaseType::Varchar(length) => format!("VARCHAR2({length})"),
                BaseType::Char(length) => format!("CHAR({length})"),
                BaseType::Float => "BINARY_FLOAT".into(),
                BaseType::Double => "BINARY_DOUBLE".into(),
                BaseType::Boolean => "BOOLEAN".into(),
                BaseType::Date => "DATE".into(),
                BaseType::DateTime => "TIMESTAMP(3)".into(),
                BaseType::Binary => "BLOB".into(),
                BaseType::UUID => "UUID".into(),
                BaseType::Json => "JSON".into(),
                BaseType::Custom(sql) => (*sql).into(),
                BaseType::Foreign(namespace, table, fields) => format!(
                    "NUMBER(10,0) REFERENCES {} ({})",
                    qualified(namespace.as_deref().or(schema), table),
                    columns(&fields.0)
                ),
                unsupported => panic!("Unsupported Oracle fixture column type: {unsupported:?}"),
            }
        );

        if tpe.increments || matches!(tpe.inner, BaseType::Primary | BaseType::Serial) {
            assert!(
                tpe.default.is_none(),
                "Auto-increment fixture cannot also specify a default"
            );
            SEQUENCES.with_borrow_mut(|state| {
                let sequence = qualified(schema, &format!("{}_{}_seq", state.table, name));
                state.before.push_str(&format!("CREATE SEQUENCE {sequence};"));
                state.after.push_str(&format!(
                    "ALTER SEQUENCE {sequence} OWNED BY {}.{};",
                    qualified(schema, &state.table),
                    ident(name)
                ));
                sql.push_str(&format!(" DEFAULT nextval('{}')", sequence.replace('\'', "''")));
            });
        } else if let Some(default) = &tpe.default {
            let value = match default {
                WrappedDefault::AnyText(text) | WrappedDefault::Custom(text) => {
                    format!("'{}'", text.replace('\'', "''"))
                }
                WrappedDefault::UUID(text) => format!("'{}'", text.replace('\'', "''")),
                WrappedDefault::Function(AutogenFunction::CurrentTimestamp) => "CURRENT_TIMESTAMP".into(),
                WrappedDefault::Boolean(value) => if *value { "true" } else { "false" }.into(),
                WrappedDefault::Integer(_)
                | WrappedDefault::Float(_)
                | WrappedDefault::Double(_)
                | WrappedDefault::Null => default.to_string(),
                unsupported => panic!("Unsupported Oracle fixture default: {unsupported:?}"),
            };
            sql.push_str(&format!(" DEFAULT {value}"));
        }
        if tpe.primary || matches!(tpe.inner, BaseType::Primary) {
            sql.push_str(" PRIMARY KEY");
        }
        if !tpe.nullable {
            sql.push_str(" NOT NULL");
        }
        if tpe.unique {
            sql.push_str(" UNIQUE");
        }
        sql
    }

    fn drop_column(name: &str) -> String {
        format!("DROP COLUMN {}", ident(name))
    }

    fn rename_column(old: &str, new: &str) -> String {
        format!("RENAME COLUMN {} TO {}", ident(old), ident(new))
    }

    fn create_index(table: &str, schema: Option<&str>, name: &str, tpe: &Type) -> String {
        let BaseType::Index(fields) = &tpe.inner else {
            unreachable!()
        };
        format!(
            "CREATE {}INDEX {} ON {} ({})",
            if tpe.unique { "UNIQUE " } else { "" },
            ident(name),
            qualified(schema, table),
            columns(fields)
        )
    }

    fn create_constraint(name: &str, tpe: &Type, schema: Option<&str>) -> String {
        let BaseType::Constraint(constraint, fields) = &tpe.inner else {
            unreachable!()
        };
        let mut sql = format!("CONSTRAINT {} {constraint} ({})", ident(name), columns(fields));
        if let Constraint::ForeignKey {
            table,
            foreign_columns,
            on_delete,
            on_update,
        } = constraint
        {
            sql.push_str(&format!(
                " REFERENCES {} ({})",
                qualified(schema, table),
                columns(foreign_columns)
            ));
            if let Some(action) = on_delete {
                sql.push_str(&format!(" ON DELETE {action}"));
            }
            if let Some(action) = on_update {
                sql.push_str(&format!(" ON UPDATE {action}"));
            }
        }
        sql
    }

    fn drop_index(name: &str) -> String {
        format!("DROP INDEX {}", ident(name))
    }

    fn add_foreign_key(fields: &[String], table: &str, references: &[String], schema: Option<&str>) -> String {
        format!(
            "FOREIGN KEY ({}) REFERENCES {} ({})",
            columns(fields),
            qualified(schema, table),
            columns(references)
        )
    }

    fn add_primary_key(fields: &[String]) -> String {
        format!("PRIMARY KEY ({})", columns(fields))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barrel::types;

    #[test]
    fn sequences_are_owned_by_the_correct_table_and_do_not_leak_between_renders() {
        let mut migration = Migration::new();
        for table in ["Parent", "Child"] {
            migration.create_table(table, |table| {
                table.add_column("id", types::primary());
            });
        }
        let sql = render(&migration);
        for table in ["Parent", "Child"] {
            assert!(sql.contains(&format!("CREATE SEQUENCE \"{table}_id_seq\";")));
            assert!(sql.contains(&format!("DEFAULT nextval('\"{table}_id_seq\"')")));
            assert!(sql.contains(&format!(
                "ALTER SEQUENCE \"{table}_id_seq\" OWNED BY \"{table}\".\"id\";"
            )));
        }
        assert!(sql.contains("NUMBER(10,0)"));
        assert!(!sql.contains("SERIAL"));
        assert!(!sql.contains("IDENTITY"));
        assert_eq!(sql, render(&migration));
    }

    #[test]
    fn quotes_identifiers_and_string_defaults() {
        let mut migration = Migration::new();
        migration.create_table("A\"Table", |table| {
            table.add_column("a\"column", types::varchar(32).default("it's fine"));
        });
        let sql = render(&migration);
        assert!(sql.contains("\"A\"\"Table\""));
        assert!(sql.contains("\"a\"\"column\" VARCHAR2(32) DEFAULT 'it''s fine'"));
    }

    #[test]
    #[should_panic(expected = "Unsupported Oracle fixture column type")]
    fn unsupported_fixture_types_are_not_silently_rendered_as_postgres() {
        let mut migration = Migration::new();
        migration.create_table("Unsupported", |table| {
            table.add_column("time", types::time());
        });
        render(&migration);
    }
}
