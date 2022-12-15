use indoc::indoc;
use migration_engine_tests::test_api::*;
use sql_schema_describer::ColumnTypeFamily;

const SCHEMA: &str = r#"
model Cat {
    id Int @id
    boxId Int?
    box Box? @relation(fields: [boxId], references: [id])
}

model Box {
    id Int @id
    material String
    cats     Cat[]
}
"#;

#[test_connector]
fn schema_push_happy_path(api: TestApi) {
    api.schema_push_w_datasource(SCHEMA)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })
        .assert_table("Box", |table| {
            table.assert_column("material", |col| col.assert_type_family(ColumnTypeFamily::String))
        });

    let dm2 = r#"
    model Cat {
        id Int @id
        boxId Int?
        residence Box? @relation(fields: [boxId], references: [id])
    }

    model Box {
        id Int @id
        texture String
        waterProof Boolean
        cats       Cat[]
    }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })
        .assert_table("Box", |table| {
            table
                .assert_columns_count(3)
                .assert_column("texture", |col| col.assert_type_family(ColumnTypeFamily::String))
        });
}

#[test_connector]
fn schema_push_warns_about_destructive_changes(api: TestApi) {
    api.schema_push_w_datasource(SCHEMA)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
        }
    "#;

    let expected_warning = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Box")
    );

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_warnings(&[expected_warning.as_str().into()])
        .assert_no_steps();

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(&[expected_warning.as_str().into()])
        .assert_has_executed_steps();
}

#[test_connector]
fn schema_push_with_an_unexecutable_migration_returns_a_message_and_aborts(api: TestApi) {
    api.schema_push_w_datasource(SCHEMA)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
            boxId Int?
            box Box? @relation(fields: [boxId], references: [id])
        }

        model Box {
            id Int @id
            material String
            volumeCm3 Int
            cats      Cat[]
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_unexecutable(&["Added the required column `volumeCm3` to the `Box` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".into()])
        .assert_no_steps();
}

#[test_connector]
fn indexes_and_unique_constraints_on_the_same_field_do_not_collide(api: TestApi) {
    let dm = r#"
        model User {
            id     BigInt    @id @default(autoincrement())
            email  String    @unique
            name   String

            @@index([email])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();
}

#[test_connector]
fn multi_column_indexes_and_unique_constraints_on_the_same_fields_do_not_collide(api: TestApi) {
    let dm = r#"
        model User {
            id     BigInt    @id @default(autoincrement())
            email  String
            name   String

            @@index([email, name])
            @@unique([email, name])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();
}

#[test_connector(exclude(Vitess))]
fn alter_constraint_name_push(api: TestApi) {
    let plain_dm = r#"
         model A {
           id   Int    @id
           name String @unique
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b])
           @@index([a])
         }

         model B {
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id])
           @@index([a,b])
           @@id([a, b])
         }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
    let no_named_pk = api.is_sqlite() || api.is_mysql();

    let (singular_id, compound_id) = if no_named_pk {
        ("", "")
    } else {
        (r#"(map: "CustomId")"#, r#", map: "CustomCompoundId""#)
    };

    let no_named_fk = if api.is_sqlite() { "" } else { r#", map: "CustomFK""# };

    let custom_dm = format!(
        r#"
         model A {{
           id   Int    @id{}
           name String @unique(map: "CustomUnique")
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b], name: "compound", map:"CustomCompoundUnique")
           @@index([a], map: "CustomIndex")
         }}
         model B {{
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id]{})
           @@index([a,b], map: "AnotherCustomIndex")
           @@id([a, b]{})
         }}
     "#,
        singular_id, no_named_fk, compound_id
    );

    api.schema_push_w_datasource(custom_dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        if !no_named_pk {
            table.assert_pk(|pk| pk.assert_constraint_name("CustomId"));
        };
        table.assert_has_index_name_and_type("CustomUnique", true);
        table.assert_has_index_name_and_type("CustomCompoundUnique", true);
        table.assert_has_index_name_and_type("CustomIndex", false)
    });

    api.assert_schema().assert_table("B", |table| {
        if !no_named_pk {
            table.assert_pk(|pk| pk.assert_constraint_name("CustomCompoundId"));
        };
        if !api.is_sqlite() {
            table.assert_fk_with_name("CustomFK");
        }
        table.assert_has_index_name_and_type("AnotherCustomIndex", false)
    });
}

#[test_connector(tags(Sqlite))]
fn sqlite_reserved_name_space_can_be_used(api: TestApi) {
    let plain_dm = r#"
         model A {
           name         String @unique(map: "sqlite_unique")
           lastName     String
           
           @@unique([name, lastName], map: "sqlite_compound_unique")
           @@index([lastName], map: "sqlite_index")
         }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
    api.assert_schema().assert_table("A", |table| {
        table.assert_has_index_name_and_type("sqlite_unique", true);
        table.assert_has_index_name_and_type("sqlite_compound_unique", true);
        table.assert_has_index_name_and_type("sqlite_index", false)
    });
}

//working constraint names

//MSSQL
#[test_connector(tags(Mssql))]
fn duplicate_index_names_across_models_work_on_mssql(api: TestApi) {
    let plain_dm = r#"
            model Post {
              id        Int     @id @default(5)
              test      Int
              
              @@index([test], map: "Duplicate")
            }
            
             model Post2 {
              id        Int     @id @default(5, map: "Duplicate")
              test      Int
              
              @@index([test], map: "Duplicate")
            }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
    api.assert_schema()
        .assert_table("Post", |table| table.assert_has_index_name_and_type("Duplicate", false));
    api.assert_schema().assert_table("Post2", |table| {
        table.assert_has_index_name_and_type("Duplicate", false)
    });
}

#[test_connector(tags(Mssql))]
fn duplicate_constraint_names_across_namespaces_work_on_mssql(api: TestApi) {
    let plain_dm = r#"
     model User {
        id         Int @id
        neighborId Int @default(1, map: "MyName")
        posts      Post[]

        @@index([id], name: "MyName")
     }

     model Post {
        id Int @id
        userId Int
        User   User @relation(fields:[userId], references:[id], map: "MyOtherName") 

        @@index([id], name: "MyOtherName")
     }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
}

#[test_connector(tags(Mssql))]
fn duplicate_primary_and_index_name_in_different_table_works_on_mssql(api: TestApi) {
    let plain_dm = r#"
     model User {
        id         Int @id(map: "Test")
     }

     model Post {
        id Int @id 
        
        @@index([id], map: "Test")
     }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
}

//Postgres

#[test_connector(tags(Postgres))]
fn duplicate_primary_and_foreign_key_name_across_models_work_on_postgres(api: TestApi) {
    let plain_dm = r#"
            model A {
                id Int @id(map: "foo")
                bs B[]
            }
            
            model B {
                id Int @id
                aId Int
                a   A  @relation(fields: [aId], references: [id], map: "foo")
            }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
}

//Mysql
#[test_connector(tags(Mysql))]
fn duplicate_constraint_names_across_models_work_on_mysql(api: TestApi) {
    let plain_dm = r#"
     model User {
        id         Int @id

        @@index([id], name: "MyName")
     }

     model Post {
        id Int @id

        @@index([id], name: "MyName")
     }
     "#;

    api.schema_push_w_datasource(plain_dm).send().assert_green();
}

#[test_connector(tags(Postgres))]
fn implicit_relations_indices_are_not_renamed_unnecessarily(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
     model UserThisIsWayTooLongAndWillLeadToProblemsDownTheRoad {
        id          Int @id
        posts       PostThisIsWayTooLongAndWillLeadToProblemsDownTheRoad[]
     }

     model PostThisIsWayTooLongAndWillLeadToProblemsDownTheRoad {
        id          Int @id
        users       UserThisIsWayTooLongAndWillLeadToProblemsDownTheRoad[]
     }
     "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("initial", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    api.create_migration("no_op", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector(tags(Mysql))]
fn creating_index_on_long_varchar_without_length_fails(api: TestApi) {
    let plain_dm = r#"
     model User {
        id         String @db.VarChar(2000)

        @@index([id])
        @@unique([id])
        @@id([id])
     }
     "#;

    api.schema_push_w_datasource(plain_dm).send_unwrap_err();
}

#[test_connector(tags(Mysql))]
fn mysql_should_diff_column_ordering_correctly_issue_10983(api: TestApi) {
    // https://github.com/prisma/prisma/issues/10983

    let dm = indoc! {r#"
        model a {
          id Int       @id @default(autoincrement())
          b  DateTime? @db.DateTime(6)

          @@index([b], map: "IDX_b")
        }
    "#};

    let ddl = r#"
        CREATE TABLE `a` (
          `id` int(11) NOT NULL AUTO_INCREMENT,
          `b` datetime(6) DEFAULT NULL,
          PRIMARY KEY (`id`),
          KEY `IDX_b` (`b`)
        )
    "#;

    api.raw_cmd(ddl);
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector]
fn issue_repro_extended_indexes(api: TestApi) {
    // https://github.com/prisma/prisma/issues/11631

    let dm = indoc! {r#"
         model HouseholdTaxSettings {
          householdId String 
          taxYear     Int
        
          @@unique([householdId, taxYear])
          @@index([householdId])
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_executable();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test]
fn multi_schema_not_implemented_on_mysql() {
    test_setup::only!(Mysql ; exclude: Vitess);

    if cfg!(windows) {
        return;
    }

    let schema = r#"
generator client {
    provider = "prisma-client-js"
    previewFeatures = ["multiSchema"]
}

datasource db {
    provider = "mysql"
    url = env("TEST_DATABASE_URL")
    schemas = ["s1", "s2"]
}

model m1 {
  id Int @id
  @@schema("s2")
}
    "#;

    let api = migration_core::migration_api(Some(schema.to_owned()), None).unwrap();
    let err = tok(api.schema_push(migration_core::json_rpc::types::SchemaPushInput {
        force: false,
        schema: schema.to_owned(),
    }))
    .unwrap_err();

    let expected = expect_test::expect![[r#"
        The `mysql` database is a system database, it should not be altered with prisma migrate. Please connect to another database.
           0: migration_core::state::SchemaPush
                     at migration-engine/core/src/state.rs:398"#]];
    expected.assert_eq(&err.to_string());
}
