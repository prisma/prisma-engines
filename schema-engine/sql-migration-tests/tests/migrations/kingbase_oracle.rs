use schema_core::schema_connector::Namespaces;
use sql_migration_tests::test_api::*;
use sql_schema_describer::{ColumnTypeFamily, ForeignKeyAction};

#[test_connector(tags(KingbaseOracle), namespaces("APP", "App", "zzz"))]
fn oracle_schema_case_migrations_preserve_distinct_namespaces(mut api: TestApi) {
    let initial = format!(
        "{}\n{}\n{}",
        api.datasource_block_with(&[("schemas", r#"["APP", "App", "zzz"]"#)]),
        api.generator_block(),
        r#"
        model UpperItem {
            id Int @id
            state State @default(ready)
            @@schema("APP")
        }
        model MixedItem {
            id Int @id
            @@schema("App")
        }
        model OtherItem {
            id Int @id
            @@schema("zzz")
        }
        enum State {
            ready
            done
            @@schema("APP")
        }
        "#,
    );
    let directory = api.create_migrations_directory();
    api.create_migration("01_initial", &initial, &directory).send_sync();
    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["01_initial"]);
    api.raw_cmd(
        r#"INSERT INTO "APP"."UpperItem" (id) VALUES (7);
           INSERT INTO "App"."MixedItem" VALUES (8);
           INSERT INTO "zzz"."OtherItem" VALUES (9);"#,
    );
    api.schema_push(&initial).send().assert_green().assert_no_steps();
    let next = initial.replacen(
        "state State @default(ready)",
        "state State @default(ready)\n note String?",
        1,
    );
    // Replaying the first migration in a shadow database must also resolve APP.
    api.create_migration("02_add_note", &next, &directory).send_sync();
    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["02_add_note"]);
    api.schema_push(&next).send().assert_green().assert_no_steps();
    api.schema_push(next.replace("\"APP\"", "\"app\""))
        .send()
        .assert_green()
        .assert_no_steps();
    for (namespace, table, id) in [
        ("APP", "UpperItem", 7),
        ("App", "MixedItem", 8),
        ("zzz", "OtherItem", 9),
    ] {
        let row = api
            .query_raw(
                &format!(r#"SELECT CAST(id AS BIGINT) AS id FROM "{namespace}"."{table}""#),
                &[],
            )
            .into_single()
            .unwrap();
        assert_eq!(row.get("id").unwrap().as_integer(), Some(id));
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_schema_case_url_default_namespace_roundtrips(api: TestApi) {
    for namespace in ["URLAPP", "UrlApp"] {
        api.raw_cmd(&format!(r#"CREATE SCHEMA "{namespace}""#));
        let mut url: url::Url = api.connection_string().parse().unwrap();
        url.query_pairs_mut().append_pair("schema", namespace);
        let mut engine = api.new_engine_with_connection_strings(url.to_string(), None);
        let schema = api.datamodel_with_provider(
            r#"
            model UrlItem {
                id Int @id
            }
            "#,
        );
        engine
            .schema_push(&schema)
            .send()
            .assert_green()
            .assert_has_executed_steps();
        api.raw_cmd(&format!(r#"INSERT INTO "{namespace}"."UrlItem" VALUES (7)"#));
        engine.schema_push(&schema).send().assert_green().assert_no_steps();
        let row = api
            .query_raw(
                &format!(r#"SELECT CAST(id AS BIGINT) AS id FROM "{namespace}"."UrlItem""#),
                &[],
            )
            .into_single()
            .unwrap();
        assert_eq!(row.get("id").unwrap().as_integer(), Some(7));
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_repair_migration_lock_excludes_other_connections(api: TestApi) {
    use quaint::{prelude::Queryable, single::Quaint};
    use schema_core::schema_connector::{ConnectorParams, SchemaConnector};
    use sql_schema_connector::SqlSchemaConnector;
    tok(async {
        let mut connector = SqlSchemaConnector::new_kingbase_oracle(ConnectorParams {
            connection_string: api.connection_string().into(),
            preview_features: Default::default(),
            shadow_database_connection_string: None,
        })
        .unwrap();
        connector.acquire_lock().await.unwrap();
        let probe = Quaint::new(api.connection_string()).await.unwrap();
        let sql = "SELECT sys_catalog.sys_try_advisory_lock(72707369) AS acquired";
        let row = probe.query_raw(sql, &[]).await.unwrap().into_single().unwrap();
        assert_eq!(row.get("acquired").unwrap().as_bool(), Some(false));
        drop(connector);
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let row = probe.query_raw(sql, &[]).await.unwrap().into_single().unwrap();
                if row.get("acquired").unwrap().as_bool() == Some(true) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        probe
            .raw_cmd("SELECT sys_catalog.sys_advisory_unlock(72707369)")
            .await
            .unwrap();
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_repair_number_normalization_preserves_values_and_is_idempotent(mut api: TestApi) {
    let schema = r#"
        model NumberRepair {
            id Int @id
            value Decimal? @db.Number(2,3)
            small Decimal? @db.Number(4,5)
        }
    "#;
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "NumberRepair" VALUES (1,0.012,0.01234),(2,NULL,NULL)"#);
    api.assert_schema().assert_table("NumberRepair", |table| {
        table
            .assert_column("value", |column| {
                column.assert_native_type("Number(3,3)", psl::builtin_connectors::KINGBASE_ORACLE)
            })
            .assert_column("small", |column| {
                column.assert_native_type("Number(5,5)", psl::builtin_connectors::KINGBASE_ORACLE)
            })
    });
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
    let canonical = schema
        .replace("Number(2,3)", "Number(3,3)")
        .replace("Number(4,5)", "Number(5,5)");
    api.schema_push_w_datasource(&canonical)
        .send()
        .assert_green()
        .assert_no_steps();
    let rows = api.query_raw(
        r#"SELECT CAST(value AS VARCHAR2(40)) AS v, CAST(small AS VARCHAR2(40)) AS s FROM "NumberRepair" ORDER BY id"#,
        &[],
    );
    assert_eq!(rows.get(0).unwrap().get("v").unwrap().as_str(), Some("0.012"));
    assert_eq!(rows.get(0).unwrap().get("s").unwrap().as_str(), Some("0.01234"));
    assert!(rows.get(1).unwrap().get("v").unwrap().is_null());
    assert!(rows.get(1).unwrap().get("s").unwrap().is_null());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_repair_migration_replay_and_binary_defaults(mut api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        model BinaryRepair {
            id Int @id @default(autoincrement())
            payload Bytes @default("q80=") @db.Blob
            state RepairState @default(READY)
            name String @unique
        }
        enum RepairState {
            READY
            DONE
        }
    "#,
    );
    let directory = api.create_migrations_directory();
    let path = api
        .create_migration("binary_repair", &schema, &directory)
        .send_sync()
        .migration_script_path();
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("decoding('abcd', 'hex')")
    );
    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["binary_repair"]);
    api.raw_cmd(r#"INSERT INTO "BinaryRepair" (name) VALUES ('kept')"#);
    let rows = api.query_raw(r#"SELECT encode(payload,'hex') AS value FROM "BinaryRepair""#, &[]);
    assert_eq!(rows.get(0).unwrap().get("value").unwrap().as_str(), Some("abcd"));
    api.schema_push(&schema).send().assert_green().assert_no_steps();
    let history = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();
    assert!(history.drift.is_none());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_repair_owned_and_standalone_indexes_drop_correctly(mut api: TestApi) {
    let schema = r#"
        model IndexRepair {
            id Int @id(map: "repair_primary")
            ordinary String
            owned String @unique(map: "repair_owned")
            standalone String
            @@index([ordinary(sort: Desc)], map: "repair_desc")
            @@unique([standalone(sort: Desc)], map: "repair_standalone")
        }
    "#;
    api.schema_push_w_datasource(schema).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "IndexRepair" VALUES(1,'a','kept','unique')"#);
    let next = schema
        .replace(" @unique(map: \"repair_owned\")", "")
        .replace("@@index([ordinary(sort: Desc)], map: \"repair_desc\")", "")
        .replace("@@unique([standalone(sort: Desc)], map: \"repair_standalone\")", "");
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "IndexRepair" VALUES(2,'a','kept','unique')"#);
    let rows = api.query_raw(r#"SELECT COUNT(*)::INTEGER AS n FROM "IndexRepair""#, &[]);
    assert_eq!(rows.get(0).unwrap().get("n").unwrap().as_integer(), Some(2));
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_failed_migration_rolls_back_ddl_without_schema_drift(api: TestApi) {
    let directory = api.create_migrations_directory();
    let initial = api.datamodel_with_provider("model Cat {\n id Int @id\n}");
    api.create_migration("initial", &initial, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    let changed = api.datamodel_with_provider("model Cat {\n id Int @id\n}\nmodel Dog {\n id Int @id\n}");
    let mut original = String::new();
    let migration_path = api
        .create_migration("add_dog", &changed, &directory)
        .send_sync()
        .modify_migration(|script| {
            original.push_str(script);
            script.push_str("\nSELECT YOLO;");
        })
        .migration_script_path();

    let error = api.apply_migrations(&directory).send_unwrap_err().to_string();
    assert!(error.to_ascii_lowercase().contains("yolo"), "{error}");
    api.assert_schema().assert_has_table("Cat").assert_has_no_table("Dog");

    std::fs::write(migration_path, original).unwrap();
    let history = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();
    assert!(history.drift.is_none());
    assert_eq!(history.failed_migration_names.len(), 1);
    assert_eq!(history.edited_migration_names.len(), 1);
    assert!(history.error_in_unapplied_migration.is_none());
}

#[test_connector(tags(KingbaseOracle), namespaces("one", "two"))]
fn schema_push_tracks_all_configured_namespaces(mut api: TestApi) {
    let datasource = api.datasource_block_with(&[("schemas", r#"["one", "two"]"#)]);
    let generator = api.generator_block();
    let schema = format!(
        r#"
            {datasource}

            {generator}

            model First {{
                id Int @id

                @@schema("one")
            }}

            model Second {{
                id Int @id

                @@schema("two")
            }}
        "#
    );

    api.schema_push(schema.clone())
        .send()
        .assert_green()
        .assert_has_executed_steps();

    let namespaces = Namespaces::from_vec(&mut vec!["one".to_owned(), "two".to_owned()]);
    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table_with_ns("one", "First")
        .assert_has_table_with_ns("two", "Second");

    api.schema_push(schema).send().assert_green().assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn changing_an_enum_preserves_replaced_column_defaults(api: TestApi) {
    let initial_schema = r#"
        model Post {
            id     Int    @id
            status Status @default(DRAFT)
        }

        enum Status {
            DRAFT
            PUBLISHED
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let changed_schema = r#"
        model Post {
            id     Int    @id
            status Status @default(ARCHIVED)
        }

        enum Status {
            ARCHIVED
            PUBLISHED
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(&[
            "The values [DRAFT] on the enum `Status` will be removed. If these variants are still used in the database, this will fail."
                .into(),
        ])
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Post", |table| {
        table.assert_column("status", |column| column.assert_enum_default("ARCHIVED"))
    });

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn schema_push_creates_oracle_native_types_and_is_idempotent(api: TestApi) {
    let schema = r#"
        model Account {
            id        Int      @id
            code      String   @unique @db.VarChar2(64)
            amount    Decimal  @db.Number(12, 4)
            createdAt DateTime @db.Timestamp(3) @default(now())
            payload   Json
            data      Bytes    @db.Blob
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))
            .assert_column("code", |column| column.assert_type_family(ColumnTypeFamily::String))
            .assert_column("amount", |column| column.assert_type_family(ColumnTypeFamily::Decimal))
            .assert_column("payload", |column| column.assert_type_family(ColumnTypeFamily::Json))
            .assert_column("data", |column| column.assert_type_family(ColumnTypeFamily::Binary))
            .assert_index_on_columns(&["code"], |index| index.assert_is_unique())
    });

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_migrations_can_be_created_applied_and_reapplied(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = api.datamodel_with_provider(
        r#"
            model Account {
                id      Int    @id
                code    String @unique @db.VarChar2(64)
                balance Decimal @db.Number(12, 4)
            }
        "#,
    );

    api.create_migration("init", &schema, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["init"]);
    api.assert_schema().assert_table("Account", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))
            .assert_index_on_columns(&["code"], |index| index.assert_is_unique())
    });

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);

    assert!(
        api.diagnose_migration_history(&migrations_directory)
            .send_sync()
            .into_output()
            .is_empty()
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_foreign_keys_and_indexes_can_be_added_and_removed(api: TestApi) {
    let initial_schema = r#"
        model Parent {
            id Int @id
        }

        model Child {
            id       Int @id
            parentId Int
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let relational_schema = r#"
        model Parent {
            id      Int     @id
            children Child[]
        }

        model Child {
            id       Int    @id
            parentId Int
            parent   Parent @relation(fields: [parentId], references: [id], onDelete: Cascade, onUpdate: Cascade)

            @@index([parentId], map: "Child_parentId_idx")
        }
    "#;

    api.schema_push_w_datasource(relational_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Child", |table| {
        table
            .assert_foreign_keys_count(1)
            .assert_fk_on_columns(&["parentId"], |fk| {
                fk.assert_references("Parent", &["id"])
                    .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
                    .assert_referential_action_on_update(ForeignKeyAction::Cascade)
            })
            .assert_index_on_columns(&["parentId"], |index| {
                index.assert_is_not_unique().assert_name("Child_parentId_idx")
            })
    });

    for (delete, update, expected_delete, expected_update) in [
        (
            "NoAction",
            "NoAction",
            ForeignKeyAction::NoAction,
            ForeignKeyAction::NoAction,
        ),
        (
            "Cascade",
            "NoAction",
            ForeignKeyAction::Cascade,
            ForeignKeyAction::NoAction,
        ),
        (
            "Restrict",
            "NoAction",
            ForeignKeyAction::Restrict,
            ForeignKeyAction::NoAction,
        ),
    ] {
        let schema = relational_schema.replace(
            "onDelete: Cascade, onUpdate: Cascade",
            &format!("onDelete: {delete}, onUpdate: {update}"),
        );
        api.schema_push_w_datasource(&schema)
            .send()
            .assert_green()
            .assert_has_executed_steps();
        api.assert_schema().assert_table("Child", |table| {
            table.assert_fk_on_columns(&["parentId"], |fk| {
                fk.assert_referential_action_on_delete(expected_delete)
                    .assert_referential_action_on_update(expected_update)
            })
        });
        api.schema_push_w_datasource(&schema)
            .send()
            .assert_green()
            .assert_no_steps();
    }

    api.schema_push_w_datasource(initial_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Child", |table| {
        table.assert_foreign_keys_count(0).assert_indexes_count(0)
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_autoincrement_columns_round_trip_through_schema_push(api: TestApi) {
    let schema = r#"
        model Event {
            id   Int    @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Event", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });

    api.insert("Event").value("name", "created").result_raw();
    api.query_raw(r#"SELECT CAST("id" AS VARCHAR2(32)) AS "id" FROM "Event""#, &[])
        .assert_single_row(|row| row.assert_text_value("id", "1"));

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_identity_bigint_widening(mut api: TestApi) {
    use psl::{SourceFile, parser_database::NoExtensionTypes};
    use schema_core::schema_connector::DiffTarget;

    let initial = r#"
        model IdentityItem {
            id Int @id @default(autoincrement())
            children IdentityChild[]
        }
        model IdentityChild {
            id Int @id
            parentId Int
            parent IdentityItem @relation(fields: [parentId], references: [id], onDelete: Cascade)
        }
    "#;
    let next = api.datamodel_with_provider(
        &initial
            .replacen("id Int", "id BigInt", 1)
            .replace("parentId Int", "parentId BigInt"),
    );

    for start in [7_i64, 2_147_483_646, 2_147_483_647] {
        api.raw_cmd(&format!(
            r#"CREATE TABLE "IdentityItem" (id INTEGER GENERATED BY DEFAULT AS IDENTITY (START WITH {start}) PRIMARY KEY);
               INSERT INTO "IdentityItem" DEFAULT VALUES;
               CREATE TABLE "IdentityChild" (id INTEGER PRIMARY KEY, "parentId" INTEGER NOT NULL,
                   CONSTRAINT "IdentityChild_parentId_fkey" FOREIGN KEY ("parentId") REFERENCES "IdentityItem"(id) ON DELETE CASCADE ON UPDATE CASCADE);
               INSERT INTO "IdentityChild" VALUES (1, {start});"#
        ));
        api.schema_push_w_datasource(initial)
            .send()
            .assert_green()
            .assert_no_steps();
        let sequence_before = api
            .query_raw(
                r#"SELECT CAST(sys_catalog.sys_get_serial_sequence('"IdentityItem"','id') AS TEXT) AS name"#,
                &[],
            )
            .into_single()
            .unwrap();
        let script = api.connector_diff(
            DiffTarget::Database,
            DiffTarget::Datamodel(
                vec![("schema.prisma".into(), SourceFile::from(next.clone()))],
                &NoExtensionTypes,
            ),
            None,
        );
        assert!(script.contains("MODIFY (\"id\" BIGINT)"), "{script}");
        assert!(
            !script.contains("DROP IDENTITY")
                && !script.contains("CREATE SEQUENCE")
                && !script.contains("DROP CONSTRAINT \"IdentityItem_pkey\""),
            "{script}"
        );
        api.schema_push(&next).send().assert_green().assert_has_executed_steps();
        api.schema_push(&next).send().assert_green().assert_no_steps();
        api.raw_cmd(r#"INSERT INTO "IdentityItem" DEFAULT VALUES; INSERT INTO "IdentityItem" DEFAULT VALUES;"#);
        api.raw_cmd(&format!(r#"INSERT INTO "IdentityChild" VALUES (2, {})"#, start + 2));
        let ids = api
            .query_raw(r#"SELECT CAST(id AS TEXT) AS id FROM "IdentityItem" ORDER BY id"#, &[])
            .into_iter()
            .map(|row| row.get("id").unwrap().as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            [start.to_string(), (start + 1).to_string(), (start + 2).to_string()]
        );
        let sequence_after = api
            .query_raw(
                r#"SELECT CAST(sys_catalog.sys_get_serial_sequence('"IdentityItem"','id') AS TEXT) AS name"#,
                &[],
            )
            .into_single()
            .unwrap();
        assert_eq!(sequence_before.get("name"), sequence_after.get("name"));
        api.assert_schema().assert_table("IdentityItem", |table| {
            table
                .assert_column("id", |column| column.assert_type_family(ColumnTypeFamily::BigInt))
                .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
        });
        api.assert_schema()
            .assert_table("IdentityChild", |table| table.assert_foreign_keys_count(1));
        let count = api.query_raw(r#"SELECT CAST(count(*) AS BIGINT) AS n FROM "IdentityChild" c JOIN "IdentityItem" p ON p.id=c."parentId""#, &[]).into_single().unwrap();
        assert_eq!(count.get("n").unwrap().as_integer(), Some(2));
        api.schema_push(&next).send().assert_green().assert_no_steps();
        api.raw_cmd(r#"DROP TABLE "IdentityChild""#);
        api.raw_cmd(r#"DROP TABLE "IdentityItem""#);
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_autoincrement_primary_keys_can_be_referenced_by_foreign_keys(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = api.datamodel_with_provider(
        r#"
            model IntParent {
                id       Int        @id @default(autoincrement())
                children IntChild[]
            }

            model IntChild {
                id       Int       @id @default(autoincrement())
                parentId Int
                parent   IntParent @relation(fields: [parentId], references: [id])
            }

            model BigIntParent {
                id       BigInt        @id @default(autoincrement())
                children BigIntChild[]
            }

            model BigIntChild {
                id       BigInt       @id @default(autoincrement())
                parentId BigInt
                parent   BigIntParent @relation(fields: [parentId], references: [id])
            }
        "#,
    );

    api.create_migration("init", &schema, &migrations_directory).send_sync();
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.assert_schema()
        .assert_table("IntParent", |table| {
            table
                .assert_column("id", |column| column.assert_type_family(ColumnTypeFamily::Int))
                .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
        })
        .assert_table("IntChild", |table| table.assert_foreign_keys_count(1))
        .assert_table("BigIntParent", |table| {
            table
                .assert_column("id", |column| column.assert_type_family(ColumnTypeFamily::BigInt))
                .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
        })
        .assert_table("BigIntChild", |table| table.assert_foreign_keys_count(1));

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);

    assert!(
        api.diagnose_migration_history(&migrations_directory)
            .send_sync()
            .into_output()
            .is_empty()
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_modify_nullability_preserves_rows(api: TestApi) {
    let optional = r#"
        model NullabilityProbe {
            id Int @id
            value String? @default("kept") @db.VarChar2(32)
            status NullabilityStatus?
            @@index([value])
        }
        enum NullabilityStatus {
            READY
            DONE
        }
    "#;
    let required = optional
        .replace("String?", "String")
        .replace("NullabilityStatus?", "NullabilityStatus");
    api.schema_push_w_datasource(optional).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "NullabilityProbe" (id, value, status) VALUES (1, 'original', 'READY')"#);

    api.schema_push_w_datasource(&required)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("NullabilityProbe", |table| {
        table
            .assert_column("value", |column| column.assert_is_required())
            .assert_column("status", |column| column.assert_is_required())
            .assert_index_on_columns(&["value"], |index| index.assert_is_not_unique())
    });
    api.raw_cmd(r#"INSERT INTO "NullabilityProbe" (id, status) VALUES (2, 'DONE')"#);
    api.query_raw(r#"SELECT value FROM "NullabilityProbe" WHERE id=2"#, &[])
        .assert_single_row(|row| row.assert_text_value("value", "kept"));
    api.schema_push_w_datasource(&required)
        .send()
        .assert_green()
        .assert_no_steps();

    api.schema_push_w_datasource(optional)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("NullabilityProbe", |table| {
        table
            .assert_column("value", |column| column.assert_is_nullable())
            .assert_column("status", |column| column.assert_is_nullable())
            .assert_index_on_columns(&["value"], |index| index.assert_is_not_unique())
    });
    api.query_raw(r#"SELECT value FROM "NullabilityProbe" WHERE id=1"#, &[])
        .assert_single_row(|row| row.assert_text_value("value", "original"));
    api.raw_cmd(r#"INSERT INTO "NullabilityProbe" (id, value, status) VALUES (3, NULL, NULL)"#);
    api.query_raw(r#"SELECT value, status FROM "NullabilityProbe" WHERE id=3"#, &[])
        .assert_single_row(|row| row.assert_null_value("value").assert_null_value("status"));
    api.schema_push_w_datasource(optional)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_column_type_default_and_arity_changes_are_migrated(mut api: TestApi) {
    let initial_schema = r#"
        model Task {
            id     Int     @id
            status String? @db.VarChar2(32)
            price Decimal? @db.Number(10,2)
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let changed_schema = r#"
        model Task {
            id     Int    @id
            status String @default("queued") @db.VarChar2(64)
            price Decimal? @default(2.5) @db.Number(10,2)
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.insert("Task").value("id", 1).result_raw();
    api.dump_table("Task")
        .assert_single_row(|row| row.assert_text_value("status", "queued"));
    api.query_raw(r#"SELECT CAST(price AS TEXT) AS price FROM "Task" WHERE id=1"#, &[])
        .assert_single_row(|row| row.assert_text_value("price", "2.50"));
    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();

    api.assert_schema().assert_table("Task", |table| {
        table.assert_column("status", |column| {
            column
                .assert_is_required()
                .assert_native_type("VarChar2(64)", psl::builtin_connectors::KINGBASE_ORACLE)
        })
    });

    let without_default_schema = r#"
        model Task {
            id     Int    @id
            status String @db.VarChar2(64)
            price Decimal? @db.Number(10,2)
        }
    "#;

    api.schema_push_w_datasource(without_default_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Task", |table| {
        table
            .assert_column("status", |column| column.assert_has_no_default())
            .assert_column("price", |column| column.assert_has_no_default())
    });

    api.insert("Task")
        .value("id", 2)
        .value("status", "explicit")
        .result_raw();
    api.query_raw(r#"SELECT price FROM "Task" WHERE id=2"#, &[])
        .assert_single_row(|row| row.assert_null_value("price"));
    api.query_raw(r#"SELECT CAST(price AS TEXT) AS price FROM "Task" WHERE id=1"#, &[])
        .assert_single_row(|row| row.assert_text_value("price", "2.50"));

    api.schema_push_w_datasource(without_default_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_named_constraints_and_indexes_can_be_renamed(api: TestApi) {
    let initial_schema = r#"
        model Parent {
            id       Int    @id(map: "Parent_pkey_old")
            code     String @unique(map: "Parent_code_key_old")
            children Child[]
        }

        model Child {
            id       Int    @id(map: "Child_pkey_old")
            parentId Int
            parent   Parent @relation(map: "Child_parent_fkey_old", fields: [parentId], references: [id])

            @@index([parentId], map: "Child_parent_idx_old")
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let changed_schema = r#"
        model Parent {
            id       Int    @id(map: "Parent_pkey_new")
            code     String @unique(map: "Parent_code_key_new")
            children Child[]
        }

        model Child {
            id       Int    @id(map: "Child_pkey_new")
            parentId Int
            parent   Parent @relation(map: "Child_parent_fkey_new", fields: [parentId], references: [id])

            @@index([parentId], map: "Child_parent_idx_new")
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table("Parent", |table| {
            table
                .assert_pk(|pk| pk.assert_constraint_name("Parent_pkey_new"))
                .assert_index_on_columns(&["code"], |index| index.assert_name("Parent_code_key_new"))
        })
        .assert_table("Child", |table| {
            table
                .assert_pk(|pk| pk.assert_constraint_name("Child_pkey_new"))
                .assert_fk_with_name("Child_parent_fkey_new")
                .assert_index_on_columns(&["parentId"], |index| index.assert_name("Child_parent_idx_new"))
        });

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_enum_columns_can_change_arity(api: TestApi) {
    let initial_schema = r#"
        model Post {
            id     Int     @id
            status Status?
        }

        enum Status {
            DRAFT
            PUBLISHED
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let changed_schema = r#"
        model Post {
            id     Int    @id
            status Status @default(DRAFT)
        }

        enum Status {
            DRAFT
            PUBLISHED
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Post", |table| {
        table.assert_column("status", |column| {
            column.assert_is_required().assert_enum_default("DRAFT")
        })
    });

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_migration_history_can_be_reset_and_reapplied(api: TestApi) {
    let initial_schema = api.datamodel_with_provider(
        r#"
            model Account {
                id   Int    @id @default(autoincrement())
                name String @db.VarChar2(64)
            }
        "#,
    );
    let changed_schema = api.datamodel_with_provider(
        r#"
            model Account {
                id   Int    @id @default(autoincrement())
                name String @db.VarChar2(64)
            }

            model AuditLog {
                id        Int      @id
                createdAt DateTime @default(now()) @db.Timestamp(3)
            }
        "#,
    );
    let migrations_directory = api.create_migrations_directory();

    api.create_migration("01_initial", &initial_schema, &migrations_directory)
        .send_sync();
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01_initial"]);
    api.create_migration("02_add_audit_log", &changed_schema, &migrations_directory)
        .send_sync();
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["02_add_audit_log"]);

    api.reset().send_sync(None);
    api.assert_schema().assert_tables_count(0);

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01_initial", "02_add_audit_log"]);
    api.assert_schema()
        .assert_has_table("Account")
        .assert_has_table("AuditLog")
        .assert_has_table("_prisma_migrations");
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_dropping_a_column_removes_its_dependent_index(api: TestApi) {
    let initial_schema = r#"
        model Item {
            id   Int    @id
            code String @db.VarChar2(32)
            name String @db.VarChar2(64)

            @@index([code, name], map: "Item_code_name_idx")
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let changed_schema = r#"
        model Item {
            id   Int    @id
            name String @db.VarChar2(64)
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Item", |table| {
        table.assert_does_not_have_column("code").assert_indexes_count(0)
    });

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_autoincrement_can_be_added_and_removed(api: TestApi) {
    let initial_schema = r#"
        model Event {
            id   Int    @id
            name String
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();

    let autoincrement_schema = r#"
        model Event {
            id   Int    @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push_w_datasource(autoincrement_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("Event", |table| {
        table
            .assert_column("id", |column| column.assert_auto_increments())
            .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });
    api.insert("Event").value("name", "generated").result_raw();
    api.dump_table("Event")
        .assert_single_row(|row| row.assert_text_value("name", "generated"));

    api.schema_push_w_datasource(initial_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("Event", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
    });

    api.schema_push_w_datasource(initial_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_table_remapping_with_existing_data_is_reported_as_data_loss(api: TestApi) {
    let initial_schema = r#"
        model Task {
            id   Int    @id
            name String @db.VarChar2(64)

            @@map("Task_old")
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Task_old")
        .value("id", 1)
        .value("name", "rename me")
        .result_raw();

    let renamed_schema = r#"
        model Task {
            id   Int    @id
            name String @db.VarChar2(64)

            @@map("Task_new")
        }
    "#;

    let warning = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Task_old")
    );

    api.schema_push_w_datasource(renamed_schema)
        .send()
        .assert_warnings(&[warning.into()])
        .assert_no_steps();
    api.assert_schema()
        .assert_has_table("Task_old")
        .assert_has_no_table("Task_new");
    api.dump_table("Task_old")
        .assert_single_row(|row| row.assert_text_value("name", "rename me"));
}

#[test_connector(tags(KingbaseOracle), namespaces("one", "two"))]
fn oracle_multi_schema_migrations_can_be_created_and_applied(mut api: TestApi) {
    let schema = api.datamodel_with_provider_and_features(
        r#"
            model First {
                id Int @id

                @@schema("one")
            }

            model Second {
                id Int @id

                @@schema("two")
            }
        "#,
        &[("schemas", r#"["one", "two"]"#)],
        &[],
    );
    let migrations_directory = api.create_migrations_directory();

    api.create_migration("initial", &schema, &migrations_directory)
        .send_sync();
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let namespaces = Namespaces::from_vec(&mut vec!["one".to_owned(), "two".to_owned()]);
    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table_with_ns("one", "First")
        .assert_has_table_with_ns("two", "Second");
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_independent_enum_values_defaults_and_rollback(mut api: TestApi) {
    let initial = r#"
        model EnumItem {
            id Int @id
            state EnumState @default(READY)
        }
        enum EnumState {
            READY
            DONE
        }
    "#;
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd("INSERT INTO \"EnumItem\"(id,state) VALUES(1,'DONE')");
    let added = initial.replace(
        "            DONE\n",
        "            DONE\n            EXTRA\n            LAST\n",
    );
    api.schema_push_w_datasource(&added).send().assert_green();
    let removed = added
        .replace("@default(READY)", "@default(EXTRA)")
        .replace("            READY\n", "");
    api.schema_push_w_datasource(&removed)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();
    assert_eq!(
        api.query_raw("SELECT state::text AS state FROM \"EnumItem\" WHERE id=1", &[])
            .first()
            .unwrap()
            .get("state")
            .unwrap()
            .to_string()
            .as_deref(),
        Some("DONE")
    );
    api.raw_cmd("INSERT INTO \"EnumItem\"(id) VALUES(2)");
    assert_eq!(
        api.query_raw("SELECT state::text AS state FROM \"EnumItem\" WHERE id=2", &[])
            .first()
            .unwrap()
            .get("state")
            .unwrap()
            .to_string()
            .as_deref(),
        Some("EXTRA")
    );
    api.schema_push_w_datasource(&removed)
        .send()
        .assert_green()
        .assert_no_steps();
    let invalid = removed.replace("            DONE\n", "");
    let error = api
        .schema_push_w_datasource(&invalid)
        .force(true)
        .send_unwrap_err()
        .to_string();
    assert!(error.to_ascii_lowercase().contains("done"), "{error}");
    api.raw_cmd("ROLLBACK");
    api.schema_push_w_datasource(&removed)
        .send()
        .assert_green()
        .assert_no_steps();
    assert_eq!(
        api.query_raw("SELECT COUNT(*) AS count FROM \"EnumItem\"", &[])
            .first()
            .unwrap()
            .get("count")
            .unwrap()
            .as_integer(),
        Some(2)
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_removed_enum_users_and_temporary_name_collisions(api: TestApi) {
    let initial = r#"
        enum State { A B }
        enum Reserved { external @@map("State_new") }
        model Item {
            id Int @id
            keep State @default(A)
            obsolete State? @default(B)
        }
        model Removed {
            id Int @id
            state State
        }
    "#
    .replace("enum State { A B }", "enum State {\n A\n B\n}")
    .replace(
        "enum Reserved { external @@map(\"State_new\") }",
        "enum Reserved {\n external\n @@map(\"State_new\")\n}",
    );
    api.schema_push_w_datasource(&initial).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "Item"(id,obsolete) VALUES(1,'B'); INSERT INTO "Removed" VALUES(1,'B');"#);
    let next = r#"
        enum State {
            A
            QUOTED @map("it's")
        }
        enum Reserved {
            external
            @@map("State_new")
        }
        model Item {
            id Int @id
            keep State @default(A)
        }
    "#;
    api.schema_push_w_datasource(next)
        .force(true)
        .send()
        .assert_has_executed_steps();
    let row = api
        .query_raw(r#"SELECT id,CAST(keep AS TEXT) AS value FROM "Item""#, &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("value").unwrap().as_str(), Some("A"));
    api.raw_cmd(r#"INSERT INTO "Item" VALUES(2,'it''s');"#);
    api.schema_push_w_datasource(next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle), namespaces("one", "two"))]
fn oracle_autoincrement_namespaces_case_and_sequence_lifetimes(mut api: TestApi) {
    let initial = r#"
        model Tenant {
            id Int @id
            @@schema("one")
        }
        model Mixed {
            id Int @id @default(autoincrement())
            @@map("Case")
            @@schema("two")
        }
        model Lower {
            id Int @id @default(autoincrement())
            @@map("case")
            @@schema("two")
        }
    "#;
    let initial = format!(
        "{}\n{}\n{initial}",
        api.datasource_block_with(&[("schemas", r#"["one","two"]"#)]),
        api.generator_block()
    );
    api.schema_push(&initial).send().assert_green();
    let next = initial.replacen("id Int @id\n", "id Int @id @default(autoincrement())\n", 1);
    api.schema_push(&next).send().assert_green().assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO one."Tenant" DEFAULT VALUES; INSERT INTO two."Case" DEFAULT VALUES; INSERT INTO two."case" DEFAULT VALUES;"#);
    let row=api.query_raw(r#"SELECT (SELECT count(*) FROM one."Tenant")+(SELECT count(*) FROM two."Case")+(SELECT count(*) FROM two."case") AS n"#,&[]).into_single().unwrap();
    assert_eq!(row.get("n").unwrap().as_integer(), Some(3));
    api.schema_push(&next).send().assert_green().assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_external_shared_owned_and_identity_generation_removal(mut api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE SEQUENCE external_seq;
        CREATE SEQUENCE owned_seq;
        CREATE SEQUENCE shared_seq;
        CREATE TABLE "External" (id INTEGER DEFAULT nextval('external_seq') PRIMARY KEY);
        CREATE TABLE "Owned" (id INTEGER DEFAULT nextval('owned_seq') PRIMARY KEY);
        ALTER SEQUENCE owned_seq OWNED BY "Owned".id;
        CREATE TABLE "SharedOwner" (id INTEGER DEFAULT nextval('shared_seq') PRIMARY KEY);
        ALTER SEQUENCE shared_seq OWNED BY "SharedOwner".id;
        CREATE TABLE "Other" (id INTEGER DEFAULT nextval('shared_seq') PRIMARY KEY);
        CREATE TABLE "NativeIdentity" (id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY);
        INSERT INTO "External" VALUES(20); INSERT INTO "Owned" VALUES(20);
        INSERT INTO "SharedOwner" VALUES(20); INSERT INTO "NativeIdentity" VALUES(20);
    "#,
    );
    let next = r#"
        model External {
            id Int @id
        }
        model Owned {
            id Int @id
        }
        model SharedOwner {
            id Int @id
        }
        model Other {
            id Int @id @default(autoincrement())
        }
        model NativeIdentity {
            id Int @id
        }
    "#;
    api.schema_push_w_datasource(next)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    for table in ["External", "Owned", "SharedOwner", "NativeIdentity"] {
        api.assert_schema().assert_table(table, |t| {
            t.assert_column("id", |c| c.assert_has_no_default().assert_no_auto_increment())
        });
        let row = api
            .query_raw(&format!("SELECT id FROM \"{table}\""), &[])
            .into_single()
            .unwrap();
        assert_eq!(row.get("id").unwrap().as_integer(), Some(20));
    }
    let rows=api.query_raw("SELECT relname FROM sys_catalog.sys_class WHERE relkind='S' AND relname IN ('external_seq','owned_seq','shared_seq') ORDER BY relname",&[]);
    let names = rows
        .into_iter()
        .map(|r| r.get("relname").unwrap().as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["external_seq", "shared_seq"]);
    api.raw_cmd(r#"INSERT INTO "Other" DEFAULT VALUES; SELECT nextval('external_seq');"#);
    api.schema_push_w_datasource(next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_distinct_physical_type_changes_are_applied(mut api: TestApi) {
    let initial = r#"
        model TypeChange {
            id Int @id
            d DateTime @db.Date
            c String @db.Clob
            f Float @db.BinaryDouble
        }
    "#;
    let next = initial
        .replace("@db.Date", "@db.Timestamp")
        .replace("@db.Clob", "@db.NClob")
        .replace("@db.BinaryDouble", "@db.Float(1)");
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "TypeChange" VALUES(1,TIMESTAMP '2026-01-02 03:04:05','value',1.25);"#);
    api.schema_push_w_datasource(&next)
        .force(true)
        .send()
        .assert_has_executed_steps();
    let rows=api.query_raw("SELECT a.attname,t.typname FROM sys_catalog.sys_attribute a JOIN sys_catalog.sys_class c ON c.oid=a.attrelid JOIN sys_catalog.sys_type t ON t.oid=a.atttypid WHERE c.relname='TypeChange' AND a.attnum>0 ORDER BY a.attname",&[]);
    let types = rows
        .into_iter()
        .map(|r| {
            (
                r.get("attname").unwrap().as_str().unwrap().to_owned(),
                r.get("typname").unwrap().as_str().unwrap().to_owned(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(types.get("d").unwrap(), "timestamp");
    assert_eq!(types.get("c").unwrap(), "nclob");
    assert_eq!(types.get("f").unwrap(), "float4");
    api.raw_cmd(r#"UPDATE "TypeChange" SET d=TIMESTAMP '2026-01-02 03:04:05.123456';"#);
    let row = api
        .query_raw(
            r#"SELECT to_char(d,'SS.US') AS fraction,c,CAST(f AS TEXT) AS f FROM "TypeChange""#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row.get("fraction").unwrap().as_str(), Some("05.123456"));
    assert_eq!(row.get("c").unwrap().as_str(), Some("value"));
    assert_eq!(row.get("f").unwrap().as_str(), Some("1.25"));
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_quoted_names_and_enum_labels_replay(mut api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        enum State {
            QUOTED @map("it's")
        }
        model Item {
            id Int @id @default(autoincrement())
            value String @map("va\"l")
            state State @default(QUOTED)
            @@map("Odd\"Name")
        }
    "#,
    );
    let directory = api.create_migrations_directory();
    api.create_migration("quoted", &schema, &directory).send_sync();
    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["quoted"]);
    api.raw_cmd(r#"INSERT INTO "Odd""Name"("va""l") VALUES('kept');"#);
    let row = api
        .query_raw(r#"SELECT "va""l" AS v,CAST(state AS TEXT) AS s FROM "Odd""Name""#, &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("v").unwrap().as_str(), Some("kept"));
    assert_eq!(row.get("s").unwrap().as_str(), Some("it's"));
    api.schema_push(&schema).send().assert_green().assert_no_steps();
    assert!(
        api.diagnose_migration_history(&directory)
            .opt_in_to_shadow_database(true)
            .send_sync()
            .into_output()
            .is_empty()
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_cast_and_json_defaults_keep_values_without_repeated_alter(mut api: TestApi) {
    let schema = r#"
        model Defaults {
            id Int @id
            amount Decimal @default(dbgenerated("1.25::numeric(2,1)")) @db.Number(10,2)
            data Json @default("{\"key\":\"value\"}")
        }
    "#;
    api.schema_push_w_datasource(schema).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "Defaults"(id) VALUES(1);"#);
    let row = api
        .query_raw(
            r#"SELECT CAST(amount AS TEXT) AS amount,CAST(data AS TEXT) AS data FROM "Defaults""#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row.get("amount").unwrap().as_str(), Some("1.30"));
    let data: serde_json::Value = serde_json::from_str(row.get("data").unwrap().as_str().unwrap()).unwrap();
    assert_eq!(data, serde_json::json!({"key":"value"}));
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_numeric_cast_defaults_r2_20261108(mut api: TestApi) {
    let schema = r#"
        model Product {
            id Int @id
            price Decimal @default(dbgenerated("CAST(1.25 AS NUMBER(10,1))")) @db.Number(10,2)
            quoted Decimal @default(dbgenerated("CAST('1.25' AS NUMBER(10,1))")) @db.Number(10,2)
            highScale Decimal @default(dbgenerated("CAST(0.12 AS NUMBER(2,4))"))
            higherScale Decimal @default(dbgenerated("CAST(0.12 AS NUMBER(38,50))"))
            roundedHighScale Decimal @default(dbgenerated("CAST(0.12345 AS NUMBER(2,4))"))
        }
    "#;
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product"(id) VALUES(1)"#);

    // Both numeric and quoted inputs must retain their effective rounding.
    let row = api
        .query_raw(
            r#"SELECT CAST(price AS TEXT) AS price, CAST(quoted AS TEXT) AS quoted FROM "Product" WHERE id=1"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row.get("price").unwrap().as_str(), Some("1.30"));
    assert_eq!(row.get("quoted").unwrap().as_str(), Some("1.30"));
    let high_scale = api
        .query_raw(
            r#"SELECT "highScale", "higherScale", "roundedHighScale" FROM "Product" WHERE id=1"#,
            &[],
        )
        .into_single()
        .unwrap();
    for (name, expected) in [
        ("highScale", "0.12"),
        ("higherScale", "0.12"),
        ("roundedHighScale", "0.1235"),
    ] {
        assert_eq!(
            high_scale.get(name).unwrap().as_numeric(),
            Some(&expected.parse().unwrap())
        );
    }

    let next = schema.replace("NUMBER(10,1)", "NUMBER(10,2)");
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product"(id) VALUES(2)"#);
    let row = api
        .query_raw(
            r#"SELECT CAST(price AS TEXT) AS price, CAST(quoted AS TEXT) AS quoted FROM "Product" WHERE id=2"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row.get("price").unwrap().as_str(), Some("1.25"));
    assert_eq!(row.get("quoted").unwrap().as_str(), Some("1.25"));

    let changed = next.replace("1.25", "1.35");
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product"(id) VALUES(3)"#);
    let row = api
        .query_raw(r#"SELECT CAST(price AS TEXT) AS price FROM "Product" WHERE id=3"#, &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("price").unwrap().as_str(), Some("1.35"));

    let changed_scale = changed.replace("CAST(0.12345 AS NUMBER(2,4))", "CAST(0.12345 AS NUMBER(5,5))");
    api.schema_push_w_datasource(&changed_scale)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&changed_scale)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product"(id) VALUES(4)"#);
    let row = api
        .query_raw(r#"SELECT "roundedHighScale" FROM "Product" WHERE id=4"#, &[])
        .into_single()
        .unwrap();
    assert_eq!(
        row.get("roundedHighScale").unwrap().as_numeric(),
        Some(&"0.12345".parse().unwrap())
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_time_and_blob_default_removal_is_idempotent(mut api: TestApi) {
    let fields = [
        ("dateValue", "DateTime", "Date", "@default(now())"),
        ("plainValue", "DateTime", "Timestamp(3)", "@default(now())"),
        ("zonedValue", "DateTime", "TimestampTz(3)", "@default(now())"),
        ("localValue", "DateTime", "TimestampLocalTz(3)", "@default(now())"),
        ("blobValue", "Bytes", "Blob", "@default(\"AQ==\")"),
    ];
    let model = |with_defaults: bool| {
        let fields = fields
            .iter()
            .map(|(name, scalar, native, expression)| {
                let default = if with_defaults {
                    format!(" {expression}")
                } else {
                    String::new()
                };
                format!("{name} {scalar}? @db.{native}{default}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("model Defaults {{\n id Int @id\n label String @db.VarChar2(20) @default(\"NULL\")\n{fields}\n}}")
    };
    let with_defaults = model(true);
    api.schema_push_w_datasource(&with_defaults)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "Defaults" (id) VALUES(1)"#);
    let without_defaults = model(false);
    api.schema_push_w_datasource(&without_defaults)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    for _ in 0..2 {
        api.schema_push_w_datasource(&without_defaults)
            .send()
            .assert_green()
            .assert_no_steps();
    }
    api.assert_schema().assert_table("Defaults", |mut table| {
        for (name, _, _, _) in fields {
            table = table.assert_column(name, |column| column.assert_has_no_default());
        }
        table
    });
    api.raw_cmd(r#"INSERT INTO "Defaults" (id) VALUES(2)"#);
    let rows = api.query_raw(r#"SELECT * FROM "Defaults" ORDER BY id"#, &[]);
    for (index, row) in rows.into_iter().enumerate() {
        assert_eq!(row.get("label").unwrap().as_str(), Some("NULL"));
        for (name, _, _, _) in fields {
            assert_eq!(row.get(name).unwrap().is_null(), index == 1, "{name}");
        }
    }
    api.schema_push_w_datasource(&with_defaults)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&with_defaults)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_character_and_negative_integer_defaults_are_idempotent(mut api: TestApi) {
    let schema = r#"
        model Defaults {
            id Int @id
            c String? @default("test") @db.Char(5)
            n Int? @default(-1)
            b BigInt? @default(-3)
        }
    "#;
    api.raw_cmd(
        r#"CREATE TABLE "Defaults" (
        id NUMBER(10,0) PRIMARY KEY, c CHAR(5) DEFAULT 'test',
        n NUMBER(10,0) DEFAULT '-1'::integer, b NUMBER(19,0) DEFAULT '-3'::integer)"#,
    );
    for id in [1, 2] {
        api.raw_cmd(&format!(r#"INSERT INTO "Defaults"(id) VALUES({id})"#));
        let row = api
            .query_raw(
                &format!(r#"SELECT c,CAST(n AS BIGINT) AS n,CAST(b AS BIGINT) AS b FROM "Defaults" WHERE id={id}"#),
                &[],
            )
            .into_single()
            .unwrap();
        assert_eq!(row.get("c").unwrap().as_str(), Some("test "));
        assert_eq!(row.get("n").unwrap().as_integer(), Some(-1));
        assert_eq!(row.get("b").unwrap().as_integer(), Some(-3));
        api.schema_push_w_datasource(schema)
            .send()
            .assert_green()
            .assert_no_steps();
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_enum_replacement_preserves_composite_foreign_keys(mut api: TestApi) {
    let initial = r#"
        enum State {
            A
            B
        }
        model EnumParent {
            tenant Int
            state State
            children EnumChild[]
            @@id([tenant,state])
        }
        model EnumChild {
            id Int @id
            tenant Int
            state State
            parent EnumParent @relation(fields:[tenant,state],references:[tenant,state],onDelete:Cascade,onUpdate:Cascade,map:"enum_foreign")
        }
    "#;
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "EnumParent" VALUES(1,'A'); INSERT INTO "EnumChild" VALUES(1,1,'A');"#);
    let next = initial.replace("            B\n", "");
    api.schema_push_w_datasource(&next)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("EnumChild", |t| {
        t.assert_fk_on_columns(&["tenant", "state"], |fk| {
            fk.assert_references("EnumParent", &["tenant", "state"])
                .assert_referential_action_on_delete(ForeignKeyAction::Cascade)
                .assert_referential_action_on_update(ForeignKeyAction::Cascade)
        })
    });
    api.raw_cmd(r#"UPDATE "EnumParent" SET tenant=2 WHERE tenant=1;"#);
    assert_eq!(
        api.query_raw(r#"SELECT CAST(tenant AS BIGINT) AS tenant FROM "EnumChild""#, &[])
            .into_single()
            .unwrap()
            .get("tenant")
            .unwrap()
            .as_integer(),
        Some(2)
    );
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"DELETE FROM "EnumParent";"#);
    assert_eq!(
        api.query_raw(r#"SELECT COUNT(*) AS n FROM "EnumChild""#, &[])
            .into_single()
            .unwrap()
            .get("n")
            .unwrap()
            .as_integer(),
        Some(0)
    );
    // The same replacement can retire an FK/table: do not restore that FK,
    // and let the ordinary drop step tolerate the already released constraint.
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd(r#"INSERT INTO "EnumParent" VALUES(1,'A'); INSERT INTO "EnumChild" VALUES(2,1,'A');"#);
    let retired = r#"
        enum State {
            A
        }
        model EnumParent {
            tenant Int
            state State
            @@id([tenant,state])
        }
    "#;
    api.schema_push_w_datasource(retired)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();
    api.assert_schema().assert_has_no_table("EnumChild");
    api.schema_push_w_datasource(retired)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_float_cast_defaults_and_wide_strings_round_trip(mut api: TestApi) {
    let schema = r#"
        model Precise {
            id Int @id
            f Float @db.BinaryDouble @default(dbgenerated("(1.23456789)::real"))
            v String @db.VarChar2(4001)
        }
    "#;
    let configured = api.datamodel_with_provider(schema);
    let dir = api.create_migrations_directory();
    api.create_migration("precise", &configured, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["precise"]);
    api.raw_cmd(r#"INSERT INTO "Precise"(id,v) VALUES(1,repeat('x',4001));"#);
    let row = api
        .query_raw(r#"SELECT CAST(f AS TEXT) AS f,length(v) AS n FROM "Precise""#, &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("f").unwrap().as_str(), Some("1.2345678806304932"));
    assert_eq!(row.get("n").unwrap().as_integer(), Some(4001));
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
    let history = api
        .diagnose_migration_history(&dir)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();
    assert!(history.drift.is_none());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_uncastable_required_columns_rebuild_only_when_empty(mut api: TestApi) {
    for (source, target, value) in [
        ("Int", "Boolean", "1"),
        ("Int", "DateTime @db.Timestamp(3)", "20261025"),
        ("DateTime @db.Timestamp(3)", "Int", "TIMESTAMP '2026-10-25 01:02:03'"),
    ] {
        let initial = format!("model Convert {{\n id Int @id\n v {source}\n}}");
        let next = format!("model Convert {{\n id Int @id\n v {target}\n}}");
        api.schema_push_w_datasource(&initial).force(true).send().assert_green();
        api.raw_cmd(&format!("INSERT INTO \"Convert\" VALUES(1,{value})"));
        for force in [false, true] {
            api.schema_push_w_datasource(&next).force(force).send().assert_unexecutable(&[format!(
                "Changed the type of `v` on the `{}` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.",api.normalize_identifier("Convert"))]).assert_no_steps();
        }
        api.raw_cmd("DELETE FROM \"Convert\"");
        api.schema_push_w_datasource(&next)
            .send()
            .assert_green()
            .assert_has_executed_steps();
        api.schema_push_w_datasource(&next)
            .send()
            .assert_green()
            .assert_no_steps();
    }
    let nullable = "model Convert {\n id Int @id\n v Int?\n}";
    let next = "model Convert {\n id Int @id\n v Boolean?\n}";
    api.schema_push_w_datasource(nullable).send().assert_green();
    api.raw_cmd("INSERT INTO \"Convert\" VALUES(1,1)");
    let warning = format!(
        "The `v` column on the `{}` table would be dropped and recreated. This will lead to data loss.",
        api.normalize_identifier("Convert")
    );
    api.schema_push_w_datasource(next)
        .send()
        .assert_warnings(&[warning.clone().into()])
        .assert_no_steps();
    api.schema_push_w_datasource(next)
        .force(true)
        .send()
        .assert_warnings(&[warning.into()])
        .assert_has_executed_steps();
    let row = api
        .query_raw("SELECT CAST(id AS BIGINT) AS id,v FROM \"Convert\"", &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("id").unwrap().as_integer(), Some(1));
    assert!(row.get("v").unwrap().is_null());
    api.schema_push_w_datasource(next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_final_autoincrement_populated_columns_round_trip(mut api: TestApi) {
    let cases = [
        ("Empty", None, 1_i64),
        ("One", Some(1_i64), 2),
        ("Ten", Some(10), 11),
        ("Negative", Some(-3), 1),
        ("Zero", Some(0), 1),
    ];
    let mut initial = String::new();
    for scalar in ["Int", "BigInt"] {
        for (suffix, _, _) in cases {
            initial.push_str(&format!(
                "model {scalar}{suffix} {{\n id {scalar} @id\n label String @default(\"new\")\n}}\n"
            ));
        }
    }
    api.schema_push_w_datasource(&initial).send().assert_green();
    for scalar in ["Int", "BigInt"] {
        for (suffix, old, _) in cases {
            if let Some(old) = old {
                api.raw_cmd(&format!(
                    "INSERT INTO \"{scalar}{suffix}\" (id,label) VALUES({old},'old')"
                ));
            }
        }
    }
    let next = initial.replace(" @id\n", " @id @default(autoincrement())\n");
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    for scalar in ["Int", "BigInt"] {
        for (suffix, old, expected) in cases {
            api.raw_cmd(&format!("INSERT INTO \"{scalar}{suffix}\" DEFAULT VALUES"));
            let rows = api.query_raw(
                &format!("SELECT CAST(id AS BIGINT) AS id,label FROM \"{scalar}{suffix}\" ORDER BY label"),
                &[],
            );
            assert_eq!(rows.get(0).unwrap().get("id").unwrap().as_integer(), Some(expected));
            assert_eq!(rows.get(0).unwrap().get("label").unwrap().as_str(), Some("new"));
            assert_eq!(rows.len(), if old.is_some() { 2 } else { 1 });
            if let Some(old) = old {
                assert_eq!(rows.get(1).unwrap().get("id").unwrap().as_integer(), Some(old));
                assert_eq!(rows.get(1).unwrap().get("label").unwrap().as_str(), Some("old"));
            }
        }
    }
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_final_autoincrement_populated_migration_replays(mut api: TestApi) {
    let initial =
        api.datamodel_with_provider("model ExistingInt {\n id Int @id\n}\nmodel ExistingBig {\n id BigInt @id\n}");
    let dir = api.create_migrations_directory();
    api.create_migration("01_initial", &initial, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["01_initial"]);
    api.raw_cmd("INSERT INTO \"ExistingInt\" VALUES(1); INSERT INTO \"ExistingBig\" VALUES(10)");
    let next = initial.replace(" @id\n", " @id @default(autoincrement())\n");
    api.create_migration("02_enable_auto", &next, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["02_enable_auto"]);
    api.raw_cmd("INSERT INTO \"ExistingInt\" DEFAULT VALUES; INSERT INTO \"ExistingBig\" DEFAULT VALUES");
    for (table, expected) in [("ExistingInt", vec![1, 2]), ("ExistingBig", vec![10, 11])] {
        let rows = api.query_raw(
            &format!("SELECT CAST(id AS BIGINT) AS id FROM \"{table}\" ORDER BY id"),
            &[],
        );
        let actual: Vec<_> = rows
            .into_iter()
            .map(|r| r.get("id").unwrap().as_integer().unwrap())
            .collect();
        assert_eq!(actual, expected);
    }
    api.schema_push(&next).send().assert_green().assert_no_steps();
    let history = api
        .diagnose_migration_history(&dir)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();
    assert!(history.drift.is_none(), "{:?}", history.drift);
    assert!(history.error_in_unapplied_migration.is_none());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_final_autoincrement_exhausted_counter_is_atomic(api: TestApi) {
    let initial = "model Exhausted {\n id BigInt @id\n}";
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd("INSERT INTO \"Exhausted\" VALUES(9223372036854775807)");
    let next = initial.replace(" @id\n", " @id @default(autoincrement())\n");
    let before = api
        .query_raw(
            "SELECT CAST(count(*) AS BIGINT) AS n FROM sys_catalog.sys_sequence",
            &[],
        )
        .into_single()
        .unwrap()
        .get("n")
        .unwrap()
        .as_integer()
        .unwrap();
    api.schema_push_w_datasource(&next).send_unwrap_err();
    api.raw_cmd("ROLLBACK");
    api.assert_schema().assert_table("Exhausted", |t| {
        t.assert_column("id", |c| c.assert_no_auto_increment().assert_has_no_default())
    });
    let value = api
        .query_raw("SELECT CAST(id AS TEXT) AS id FROM \"Exhausted\"", &[])
        .into_single()
        .unwrap();
    assert_eq!(value.get("id").unwrap().as_str(), Some("9223372036854775807"));
    let after = api
        .query_raw(
            "SELECT CAST(count(*) AS BIGINT) AS n FROM sys_catalog.sys_sequence",
            &[],
        )
        .into_single()
        .unwrap()
        .get("n")
        .unwrap()
        .as_integer()
        .unwrap();
    assert_eq!(before, after);
    api.schema_push_w_datasource(initial)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_final_enum_addition_default_migration_and_replay(mut api: TestApi) {
    let body = r#"
        enum State {
            A
            B
        }
        model Parent {
            id Int @id
            state State @unique
            children Child[]
        }
        model Child {
            id Int @id
            state State @default(A)
            parent Parent @relation(fields:[state],references:[state],onDelete:Cascade,onUpdate:Cascade,map:"state_fk")
        }
    "#;
    let initial = api.datamodel_with_provider(body);
    let dir = api.create_migrations_directory();
    api.create_migration("01_initial", &initial, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["01_initial"]);
    api.raw_cmd("INSERT INTO \"Parent\" VALUES(1,'A'),(2,'B'); INSERT INTO \"Child\" VALUES(1,'A')");
    let next = initial
        .replace("            B\n", "            B\n            C\n")
        .replace("@default(A)", "@default(C)")
        .replace(
            "            id Int @id\n            state State @default(C)",
            "            id Int @id\n            extra State @default(C)\n            state State @default(C)",
        );
    api.create_migration("02_add_c_default", &next, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["02_add_c_default"]);
    let old = api
        .query_raw(
            "SELECT state::text AS state,extra::text AS extra FROM \"Child\" WHERE id=1",
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(old.get("state").unwrap().as_str(), Some("A"));
    assert_eq!(old.get("extra").unwrap().as_str(), Some("C"));
    api.raw_cmd("INSERT INTO \"Parent\" VALUES(3,'C'); INSERT INTO \"Child\" (id) VALUES(2)");
    let row = api
        .query_raw(
            "SELECT state::text AS state,extra::text AS extra FROM \"Child\" WHERE id=2",
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row.get("state").unwrap().as_str(), Some("C"));
    assert_eq!(row.get("extra").unwrap().as_str(), Some("C"));
    // Only a new table uses D; there is no changed default on an old column.
    let next = next.replace("            C\n", "            C\n            D\n")
        + "\nmodel NewDefault {\n id Int @id\n state State @default(D)\n}\n";
    api.create_migration("03_new_table_default", &next, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["03_new_table_default"]);
    api.raw_cmd("INSERT INTO \"NewDefault\"(id) VALUES(1)");
    let row = api
        .query_raw("SELECT state::text AS state FROM \"NewDefault\"", &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("state").unwrap().as_str(), Some("D"));
    // Conservatively handle dbgenerated enum defaults without parsing SQL text.
    let next = next.replace("            D\n", "            D\n            E\n")
        + "\nmodel RawDefault {\n id Int @id\n state State @default(dbgenerated(\"'E'\"))\n}\n";
    api.create_migration("04_raw_default", &next, &dir).send_sync();
    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["04_raw_default"]);
    api.raw_cmd("INSERT INTO \"RawDefault\"(id) VALUES(1)");
    let row = api
        .query_raw("SELECT state::text AS state FROM \"RawDefault\"", &[])
        .into_single()
        .unwrap();
    assert_eq!(row.get("state").unwrap().as_str(), Some("E"));
    let canonical = next.replace("@default(dbgenerated(\"'E'\"))", "@default(E)");
    api.schema_push(&canonical).send().assert_green().assert_no_steps();
    let history = api
        .diagnose_migration_history(&dir)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();
    assert!(history.drift.is_none());
    assert!(history.error_in_unapplied_migration.is_none());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_final_autoincrement_waits_for_inflight_writes(mut api: TestApi) {
    use psl::{SourceFile, parser_database::NoExtensionTypes};
    use quaint::{prelude::Queryable, single::Quaint};
    use schema_core::schema_connector::{
        ConnectorParams, DestructiveChangeDiagnostics, SchemaConnector, SchemaDialect, SchemaFilter,
    };
    use sql_schema_connector::{SqlSchemaConnector, SqlSchemaDialect};
    let initial = "model ConcurrentAuto {\n id BigInt @id\n}";
    let next = initial.replace(" @id\n", " @id @default(autoincrement())\n");
    api.schema_push_w_datasource(initial).send().assert_green();
    api.raw_cmd("INSERT INTO \"ConcurrentAuto\" VALUES(1)");
    let dialect = SqlSchemaDialect::kingbase_oracle();
    let namespace = api.schema_name();
    let calc = |s: &str| {
        dialect
            .schema_from_datamodel(
                vec![("schema.prisma".into(), SourceFile::from(api.datamodel_with_provider(s)))],
                Some(&namespace),
                &NoExtensionTypes,
            )
            .unwrap()
    };
    let migration = dialect.diff(calc(initial), calc(&next), &SchemaFilter::default());
    let script = dialect
        .render_script(&migration, &DestructiveChangeDiagnostics::default())
        .unwrap();
    tok(async {
        let writer = Quaint::new(api.connection_string()).await.unwrap();
        let mut migrator = SqlSchemaConnector::new_kingbase_oracle(ConnectorParams::new(
            api.connection_string().into(),
            psl::PreviewFeatures::empty(),
            None,
        ))
        .unwrap();
        migrator.db_execute("SELECT 1".into()).await.unwrap();
        writer
            .raw_cmd("BEGIN; INSERT INTO \"ConcurrentAuto\" VALUES(50)")
            .await
            .unwrap();
        let mut task = tokio::spawn(async move { migrator.apply_script("enable_auto", &script).await });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(200), &mut task)
                .await
                .is_err(),
            "migration must wait for the uncommitted writer"
        );
        writer.raw_cmd("COMMIT").await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    });
    api.raw_cmd("INSERT INTO \"ConcurrentAuto\" DEFAULT VALUES");
    let rows = api.query_raw(
        "SELECT CAST(id AS BIGINT) AS id FROM \"ConcurrentAuto\" ORDER BY id",
        &[],
    );
    let actual: Vec<_> = rows
        .into_iter()
        .map(|r| r.get("id").unwrap().as_integer().unwrap())
        .collect();
    assert_eq!(actual, vec![1, 50, 51]);
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_omitted_native_type_arguments_do_not_repeat_migrations(mut api: TestApi) {
    let initial = r#"
        model Product {
            id Int @id @db.Number(10)
            large BigInt @db.Number(19)
            code String @db.Char
            national String @db.NChar
        }
    "#;
    api.schema_push_w_datasource(initial)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "Product" (id, large, code, national) VALUES (1, 9007199254740993, 'a', 'b')"#);
    api.schema_push_w_datasource(initial)
        .send()
        .assert_green()
        .assert_no_steps();
    let explicit = initial
        .replace("Number(10)", "Number(10, 0)")
        .replace("Number(19)", "Number(19, 0)")
        .replace("@db.Char", "@db.Char(1)")
        .replace("@db.NChar", "@db.NChar(1)");
    api.schema_push_w_datasource(&explicit)
        .send()
        .assert_green()
        .assert_no_steps();
    let wider = explicit.replace("@db.Char(1)", "@db.Char(2)");
    api.schema_push_w_datasource(&wider)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&wider)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_uuid_xml_literal_defaults_do_not_repeat_migrations(mut api: TestApi) {
    let initial = r#"
        model Product {
            id Int @id
            token String @db.Uuid @default("123e4567-e89b-12d3-a456-426614174000")
            payload String @db.Xml @default("<item name='test'>value</item>")
        }
    "#;
    api.schema_push_w_datasource(initial)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(initial)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product" (id) VALUES (1)"#);
    let row = api
        .query_raw(
            r#"SELECT CAST(token AS TEXT) AS token, CAST(payload AS TEXT) AS payload FROM "Product" WHERE id=1"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(
        row.get("token").unwrap().as_str(),
        Some("123e4567-e89b-12d3-a456-426614174000")
    );
    assert_eq!(
        row.get("payload").unwrap().as_str(),
        Some("<item name='test'>value</item>")
    );
    let next = initial
        .replace("426614174000", "426614174001")
        .replace("value</item>", "changed</item>");
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&next)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "Product" (id) VALUES (2)"#);
    let row = api
        .query_raw(
            r#"SELECT CAST(token AS TEXT) AS token, CAST(payload AS TEXT) AS payload FROM "Product" WHERE id=2"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(
        row.get("token").unwrap().as_str(),
        Some("123e4567-e89b-12d3-a456-426614174001")
    );
    assert_eq!(
        row.get("payload").unwrap().as_str(),
        Some("<item name='test'>changed</item>")
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_desc_unique_with_referenced_constraint_migrates(mut api: TestApi) {
    let schema = r#"
        model Parent {
            id Int @id
            code String
            label String
            children Child[]
            @@unique([code(sort: Desc), label], map: "parent_desc_index")
            @@unique([code, label], map: "parent_constraint")
        }
        model Child {
            id Int @id
            parentCode String
            parentLabel String
            parent Parent @relation(fields: [parentLabel, parentCode], references: [label, code])
        }
    "#;
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "Parent" (id, code, label) VALUES (1, 'a', 'b')"#);
    api.raw_cmd(r#"INSERT INTO "Child" (id, "parentCode", "parentLabel") VALUES (1, 'a', 'b')"#);
    api.assert_schema()
        .assert_table("Child", |table| table.assert_foreign_keys_count(1));
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_default_decimal_uses_unconstrained_number(mut api: TestApi) {
    let schema = r#"
        model NumberDefault {
            id Int @id
            value Decimal @default(1.25)
            optional Decimal?
        }
    "#;
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("NumberDefault", |table| {
        table.assert_column("value", |column| {
            column.assert_native_type("Number", psl::builtin_connectors::KINGBASE_ORACLE)
        })
    });
    api.raw_cmd(
        r#"INSERT INTO "NumberDefault" (id) VALUES (1);
        INSERT INTO "NumberDefault" VALUES (2,123456789012345678901234567890.12345678,1.2345e-100);"#,
    );
    let rows = api.query_raw(r#"SELECT value, optional FROM "NumberDefault" ORDER BY id"#, &[]);
    assert_eq!(
        rows.get(0).unwrap().get("value").unwrap().as_numeric(),
        Some(&"1.25".parse().unwrap())
    );
    assert!(rows.get(0).unwrap().get("optional").unwrap().is_null());
    assert_eq!(
        rows.get(1).unwrap().get("value").unwrap().as_numeric(),
        Some(&"123456789012345678901234567890.12345678".parse().unwrap())
    );
    assert_eq!(
        rows.get(1).unwrap().get("optional").unwrap().as_numeric(),
        Some(&"1.2345e-100".parse().unwrap())
    );
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
    let explicit = schema.replace("value Decimal @default", "value Decimal @db.Number @default");
    api.schema_push_w_datasource(&explicit)
        .send()
        .assert_green()
        .assert_no_steps();
    let changed = explicit.replace("@default(1.25)", "@default(2.5)");
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_legacy_wide_number_migrates_to_default_decimal(mut api: TestApi) {
    api.raw_cmd(r#"CREATE TABLE "NumberLegacy" (id NUMBER(10,0) PRIMARY KEY, value NUMBER(65,30) NOT NULL)"#);
    let schema = r#"
        model NumberLegacy {
            id Int @id
            value Decimal
        }
    "#;
    let unsupported = schema.replace("value Decimal", "value Unsupported(\"numeric(65,30)\")");
    for _ in 0..2 {
        api.schema_push_w_datasource(&unsupported)
            .send()
            .assert_green()
            .assert_no_steps();
    }
    // A different SQL type must still migrate, even though both are Unsupported.
    let changed = unsupported.replace("numeric(65,30)", "numeric(66,30)");
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&changed)
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(&unsupported)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    let with_default = unsupported.replace(
        "numeric(65,30)\")",
        "numeric(65,30)\") @default(dbgenerated(\"CAST(1.25 AS NUMBER(10,1))\"))",
    );
    api.schema_push_w_datasource(&with_default)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&with_default)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "NumberLegacy" (id) VALUES (10)"#);
    let changed_default = with_default.replace("CAST(1.25 AS NUMBER(10,1))", "CAST(2.5 AS NUMBER(10,1))");
    api.schema_push_w_datasource(&changed_default)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&changed_default)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "NumberLegacy" (id) VALUES (11)"#);
    api.schema_push_w_datasource(&unsupported)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("NumberLegacy", |table| {
        table.assert_column("value", |column| column.assert_has_no_default())
    });
    for _ in 0..2 {
        api.schema_push_w_datasource(&unsupported)
            .send()
            .assert_green()
            .assert_no_steps();
    }
    let defaults = api.query_raw(r#"SELECT value FROM "NumberLegacy" ORDER BY id"#, &[]);
    assert_eq!(
        defaults.get(0).unwrap().get("value").unwrap().as_numeric(),
        Some(&"1.3".parse().unwrap())
    );
    assert_eq!(
        defaults.get(1).unwrap().get("value").unwrap().as_numeric(),
        Some(&"2.5".parse().unwrap())
    );
    let high_scale = with_default.replace("CAST(1.25 AS NUMBER(10,1))", "CAST(0.12 AS NUMBER(2,4))");
    api.schema_push_w_datasource(&high_scale)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&high_scale)
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(&unsupported)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&unsupported)
        .send()
        .assert_green()
        .assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "NumberLegacy" VALUES (1,12345678901234567890.123456789)"#);
    api.schema_push_w_datasource(&unsupported)
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    let row = api
        .query_raw(r#"SELECT value FROM "NumberLegacy" WHERE id=1"#, &[])
        .into_single()
        .unwrap();
    assert_eq!(
        row.get("value").unwrap().as_numeric(),
        Some(&"12345678901234567890.123456789".parse().unwrap())
    );
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_number_high_scale_remains_idempotent(mut api: TestApi) {
    let schema = r#"
        model NumberScale {
            id Int @id
            value Decimal @db.Number(2,50)
            boundary Decimal? @db.Number(38,1000)
        }
    "#;
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(r#"INSERT INTO "NumberScale" VALUES (1,1.2345e-40,NULL)"#);
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
    let wider = schema.replace("Number(2,50)", "Number(38,60)");
    api.schema_push_w_datasource(&wider)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(&wider)
        .send()
        .assert_green()
        .assert_no_steps();
    let row = api
        .query_raw(r#"SELECT value FROM "NumberScale" WHERE id=1"#, &[])
        .into_single()
        .unwrap();
    assert_eq!(
        row.get("value").unwrap().as_numeric(),
        Some(&"1.2345e-40".parse().unwrap())
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_additional_native_types_roundtrip_without_repeated_migrations(mut api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        model AdditionalNativeTypes {
            id Int @id
            fixed String? @db.CharByte(8) @default("abc")
            variable String? @db.VarChar2Byte(8) @default("abc")
            row String? @db.RowId @default("AAAAAAAAAAABAAAAAAAAAAA")
            universal String? @db.URowId(4000) @default("AAAAAAAAAAABAAAAAAAAAAA")
            integer Int? @db.BinaryInteger @default(7)
        }
    "#,
    );
    api.schema_push(&schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(&schema).send().assert_green().assert_no_steps();
    api.raw_cmd(r#"INSERT INTO "AdditionalNativeTypes" (id, fixed, variable, "row", universal, integer) VALUES (1, '中文ab', '中文ab', nextrowid(), 'AAAAAAAAAAABAAAAAAAAAAA', -2147483648)"#);
    let row = api
        .query_raw(
            r#"SELECT fixed, variable, "row", universal, integer FROM "AdditionalNativeTypes" WHERE id = 1"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row["fixed"].as_str(), Some("中文ab"));
    assert_eq!(row["variable"].as_str(), Some("中文ab"));
    assert_eq!(row["integer"].as_integer(), Some(i64::from(i32::MIN)));
    let widened = schema
        .replace("CharByte(8)", "CharByte(12)")
        .replace("VarChar2Byte(8)", "VarChar2Byte(12)")
        .replace("URowId(4000)", "URowId(5000)");
    api.schema_push(&widened)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(&widened).send().assert_green().assert_no_steps();
    let row = api
        .query_raw(
            r#"SELECT fixed, integer FROM "AdditionalNativeTypes" WHERE id = 1"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row["fixed"].as_str(), Some("中文ab    "));
    assert_eq!(row["integer"].as_integer(), Some(i64::from(i32::MIN)));
    api.raw_cmd(r#"INSERT INTO "AdditionalNativeTypes" (id) VALUES (2)"#);
    let row = api
        .query_raw(
            r#"SELECT fixed, variable, "row", universal, integer FROM "AdditionalNativeTypes" WHERE id = 2"#,
            &[],
        )
        .into_single()
        .unwrap();
    assert_eq!(row["fixed"].as_str(), Some("abc         "));
    assert_eq!(row["variable"].as_str(), Some("abc"));
    assert_eq!(row["row"].as_str(), Some("AAAAAAAAAAABAAAAAAAAAAA"));
    assert_eq!(row["universal"].as_str(), Some("AAAAAAAAAAABAAAAAAAAAAA"));
    assert_eq!(row["integer"].as_integer(), Some(7));
}
