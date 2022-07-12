mod describers;
mod test_api;

use crate::test_api::*;
use barrel::types;
use pretty_assertions::assert_eq;
use quaint::prelude::SqlFamily;

#[test_connector]
fn is_required_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User", |t| {
            t.add_column("column1", types::integer().nullable(false));
            t.add_column("column2", types::integer().nullable(true));
        });
    });

    api.describe().assert_table("User", |t| {
        t.assert_column("column1", |c| c.assert_not_null())
            .assert_column("column2", |c| c.assert_nullable())
    });
}

#[test_connector(exclude(Sqlite))]
fn foreign_keys_must_work(api: TestApi) {
    let sql_family = api.sql_family();

    api.execute_barrel(|migration| {
        migration.create_table("City", |t| {
            t.add_column("id", types::primary());
        });
        migration.create_table("User", move |t| {
            // barrel does not render foreign keys correctly for mysql
            // TODO: Investigate
            if sql_family == SqlFamily::Mysql {
                t.add_column("city", types::integer());
                t.inject_custom("FOREIGN KEY(city) REFERENCES City(id) ON DELETE RESTRICT");
            } else {
                t.add_column("city", types::foreign("City", "id"));
            }
        });
    });

    let schema = api.describe();

    schema.assert_table("User", |t| {
        let t = t
            .assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_foreign_key_on_columns(&["city"], |fk| fk.assert_references("City", &["id"]));

        if sql_family.is_mysql() {
            t.assert_index_on_columns(&["city"], |idx| idx.assert_name("city"))
        } else {
            t
        }
    });
}

#[test_connector(exclude(Sqlite))]
fn multi_column_foreign_keys_must_work(api: TestApi) {
    let sql_family = api.sql_family();
    let schema = api.schema_name().to_owned();

    api.execute_barrel(|migration| {
        migration.create_table("City", move |t| {
            t.add_column("id", types::primary());
            t.add_column("name", types::varchar(255));
            t.inject_custom("constraint uniq unique (name, id)");
        });
        migration.create_table("User", move |t| {
            t.add_column("city", types::integer());
            t.add_column("city_name", types::varchar(255));

            if sql_family == SqlFamily::Mysql {
                t.inject_custom("FOREIGN KEY(city_name, city) REFERENCES City(name, id) ON DELETE RESTRICT");
            } else if sql_family == SqlFamily::Mssql {
                t.inject_custom(format!(
                    "FOREIGN KEY(city_name, city) REFERENCES [{}].[City]([name], [id])",
                    schema,
                ));
            } else {
                let relation_prefix = match sql_family {
                    SqlFamily::Postgres => format!("\"{}\".", &schema),
                    _ => "".to_string(),
                };
                t.inject_custom(format!(
                    "FOREIGN KEY(city_name, city) REFERENCES {}\"City\"(name, id)",
                    relation_prefix
                ));
            }
        });
    });

    let schema = api.describe();

    schema.assert_table("User", |t| {
        let t = t
            .assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_name", |c| c.assert_type_is_string())
            .assert_foreign_key_on_columns(&["city_name", "city"], |fk| {
                fk.assert_references("City", &["name", "id"])
            });

        if sql_family.is_mysql() {
            t.assert_index_on_columns(&["city_name", "city"], |idx| idx.assert_name("city_name"))
        } else {
            t
        }
    });
}

#[test_connector]
fn names_with_hyphens_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User-table", |t| {
            t.add_column("column-1", types::integer().nullable(false));
        });
    });

    api.describe().assert_table("User-table", |table| {
        table.assert_column("column-1", |c| c.assert_not_null())
    });
}

#[test_connector]
fn composite_primary_keys_must_work(api: TestApi) {
    let sql = match api.sql_family() {
        SqlFamily::Mysql => format!(
            "CREATE TABLE `{0}`.`User` (
                id INTEGER NOT NULL,
                name VARCHAR(255) NOT NULL,
                PRIMARY KEY(id, name)
            )",
            api.db_name()
        ),
        SqlFamily::Mssql => format!(
            "CREATE TABLE [{}].[User] (
                [id] INT NOT NULL,
                [name] VARCHAR(255) NOT NULL,
                CONSTRAINT [PK_User] PRIMARY KEY ([id], [name])
            )",
            api.schema_name(),
        ),
        _ => format!(
            "CREATE TABLE \"{0}\".\"User\" (
                id INTEGER NOT NULL,
                name VARCHAR(255) NOT NULL,
                PRIMARY KEY(id, name)
            )",
            api.schema_name()
        ),
    };

    api.raw_cmd(&sql);
    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    assert_eq!(table.name(), "User");
    assert_eq!(
        table
            .primary_key_columns()
            .unwrap()
            .map(|c| c.name())
            .collect::<Vec<_>>(),
        &["id", "name"]
    );
}
