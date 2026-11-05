use crate::test_api::*;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use psl::builtin_connectors::{KingbaseOracleNumberArguments, KingbaseOracleType};
use sql_schema_describer::{ColumnTypeFamily, ForeignKeyAction, SQLSortOrder};

#[test_connector(tags(KingbaseOracle))]
fn oracle_schema_case_aliases_describe_physical_names(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE SCHEMA "APP";
        CREATE SCHEMA "App";
        CREATE TYPE "APP"."State" AS ENUM ('ready','done');
        CREATE TABLE "APP"."Item" (id NUMBER(10,0) PRIMARY KEY, state "APP"."State" DEFAULT 'ready');
        CREATE TABLE "App"."Item" (id NUMBER(10,0) PRIMARY KEY);
        CREATE VIEW "APP"."ItemView" AS SELECT id FROM "APP"."Item";
        INSERT INTO "APP"."Item" (id) VALUES (7);
        INSERT INTO "App"."Item" VALUES (8);
        "#,
    );
    // Request the SQL spelling, not the already-normalized catalog spelling.
    let schema = api.describe_with_schemas(&["APP", "app", "App"]);
    assert!(schema.namespace_walker("APP").is_none());
    assert_eq!(schema.namespaces_count(), 2);
    assert!(schema.table_walker_ns("app", "Item").is_some());
    assert!(schema.table_walker_ns("App", "Item").is_some());
    assert!(schema.view_walker_ns("app", "ItemView").is_some());
    assert!(schema.find_enum("State", Some("app")).is_some());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_repair_sys_catalogs_without_pg_reads(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE catalog_repair(id NUMBER(10,0) PRIMARY KEY, value NUMBER(3,3) DEFAULT 0.012,
            name VARCHAR2(40) UNIQUE, payload BLOB DEFAULT decoding('abcd','hex'));
        CREATE INDEX catalog_repair_desc ON catalog_repair(name DESC);
        CREATE TYPE catalog_repair_state AS ENUM ('ready','done');
        ALTER TABLE catalog_repair ADD state catalog_repair_state DEFAULT 'ready';
        COMMENT ON TYPE catalog_repair_state IS 'enum metadata kept';
        ALTER TABLE catalog_repair ADD CONSTRAINT catalog_repair_positive CHECK(value > 0);
        CREATE FUNCTION catalog_repair_function() RETURNS INTEGER AS 'SELECT 1' LANGUAGE sql;
        CREATE VIEW catalog_repair_view AS SELECT id,name FROM catalog_repair;
        COMMENT ON TABLE catalog_repair IS 'table metadata kept';
        COMMENT ON COLUMN catalog_repair.name IS 'column metadata kept';
        COMMENT ON VIEW catalog_repair_view IS 'view metadata kept';
        BEGIN;
        CREATE ROLE oracle_repair_catalog_reader;
        GRANT SELECT ON catalog_repair,catalog_repair_view TO oracle_repair_catalog_reader;
        REVOKE SELECT ON pg_catalog.pg_type,pg_catalog.pg_range,pg_catalog.pg_namespace,
            pg_catalog.pg_enum,pg_catalog.pg_attribute,pg_catalog.pg_class,
            pg_catalog.pg_constraint,pg_catalog.pg_index,pg_catalog.pg_attrdef,
            pg_catalog.pg_am,pg_catalog.pg_opclass,pg_catalog.pg_description,
            pg_catalog.pg_proc,pg_catalog.pg_language,pg_catalog.pg_extension,
            pg_catalog.pg_views,pg_catalog.pg_depend FROM PUBLIC;
        SET ROLE oracle_repair_catalog_reader;
    "#,
    );
    api.raw_cmd("SAVEPOINT denied_pg");
    let denied = tok(api.database().raw_cmd("SELECT oid FROM pg_catalog.pg_type LIMIT 0")).unwrap_err();
    assert_eq!(denied.original_code(), Some("42501"));
    api.raw_cmd("ROLLBACK TO SAVEPOINT denied_pg");
    let schema = api.describe();
    api.raw_cmd("ROLLBACK");
    let table = schema.table_walker("catalog_repair").unwrap();
    assert_eq!(table.description(), Some("table metadata kept"));
    assert_eq!(
        table.column("name").unwrap().description(),
        Some("column metadata kept")
    );
    assert_eq!(
        table
            .indexes()
            .find(|index| index.name() == "catalog_repair_desc")
            .unwrap()
            .columns()
            .next()
            .unwrap()
            .sort_order(),
        Some(SQLSortOrder::Desc)
    );
    let view = schema
        .view_walkers()
        .find(|view| view.name() == "catalog_repair_view")
        .unwrap();
    assert_eq!(view.description(), Some("view metadata kept"));
    let enm = schema
        .enum_walkers()
        .find(|enm| enm.name() == "catalog_repair_state")
        .unwrap();
    assert_eq!(enm.description(), Some("enum metadata kept"));
    assert!(schema.get_procedure("catalog_repair_function").is_some());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_column_types_and_identity_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE type_samples (
            id SERIAL PRIMARY KEY,
            tinyint_value TINYINT,
            int_value NUMBER(10, 0),
            bigint_value NUMBER(19, 0),
            decimal_value NUMBER(12, 4),
            binary_float_value BINARY_FLOAT,
            binary_double_value BINARY_DOUBLE,
            char_value CHAR(10),
            varchar_value VARCHAR2(32),
            clob_value CLOB,
            nclob_value NCLOB,
            blob_value BLOB,
            date_value DATE,
            timestamp_value TIMESTAMP(3),
            timestamp_tz_value TIMESTAMP(3) WITH TIME ZONE,
            bool_value BOOLEAN,
            json_value JSON,
            uuid_value UUID,
            xml_value XML
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("type_samples").unwrap();

    assert_eq!(1, table.primary_key_columns_count());
    assert!(table.column("id").unwrap().is_autoincrement());

    for (column_name, expected_family, expected_native_type) in [
        (
            "id",
            ColumnTypeFamily::Int,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(10, 0),
            )),
        ),
        (
            "tinyint_value",
            ColumnTypeFamily::Int,
            Some(KingbaseOracleType::TinyInt),
        ),
        (
            "int_value",
            ColumnTypeFamily::Int,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(10, 0),
            )),
        ),
        (
            "bigint_value",
            ColumnTypeFamily::BigInt,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(19, 0),
            )),
        ),
        (
            "decimal_value",
            ColumnTypeFamily::Decimal,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(12, 4),
            )),
        ),
        (
            "binary_float_value",
            ColumnTypeFamily::Float,
            Some(KingbaseOracleType::BinaryFloat),
        ),
        (
            "binary_double_value",
            ColumnTypeFamily::Float,
            Some(KingbaseOracleType::BinaryDouble),
        ),
        (
            "char_value",
            ColumnTypeFamily::String,
            Some(KingbaseOracleType::Char(Some(10))),
        ),
        (
            "varchar_value",
            ColumnTypeFamily::String,
            Some(KingbaseOracleType::VarChar2(Some(32))),
        ),
        ("clob_value", ColumnTypeFamily::String, Some(KingbaseOracleType::Clob)),
        // NCLOB retains its own physical type in the Oracle dictionary.
        ("nclob_value", ColumnTypeFamily::String, Some(KingbaseOracleType::NClob)),
        ("blob_value", ColumnTypeFamily::Binary, Some(KingbaseOracleType::Blob)),
        // The Oracle dictionary and physical type preserve DATE (with time-of-day).
        // information_schema alone exposes a timestamp alias and loses this identity.
        ("date_value", ColumnTypeFamily::DateTime, Some(KingbaseOracleType::Date)),
        (
            "timestamp_value",
            ColumnTypeFamily::DateTime,
            Some(KingbaseOracleType::Timestamp(Some(3))),
        ),
        (
            "timestamp_tz_value",
            ColumnTypeFamily::DateTime,
            Some(KingbaseOracleType::TimestampTz(Some(3))),
        ),
        (
            "bool_value",
            ColumnTypeFamily::Boolean,
            Some(KingbaseOracleType::Boolean),
        ),
        ("json_value", ColumnTypeFamily::Json, Some(KingbaseOracleType::Json)),
        ("uuid_value", ColumnTypeFamily::String, Some(KingbaseOracleType::Uuid)),
        ("xml_value", ColumnTypeFamily::String, Some(KingbaseOracleType::Xml)),
    ] {
        let column = table.column(column_name).unwrap();
        assert_eq!(
            &expected_family,
            column.column_type_family(),
            "wrong type family for {column_name}"
        );
        let native_type = column
            .column_type()
            .native_type
            .as_ref()
            .map(|native_type| *native_type.downcast_ref::<KingbaseOracleType>());
        assert_eq!(expected_native_type, native_type, "wrong native type for {column_name}");
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_native_type_boundaries_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE type_boundaries (
            number_unspecified NUMBER,
            number_boolean NUMBER(1, 0),
            float_value FLOAT(53),
            national_char NCHAR(10),
            national_varchar NVARCHAR2(32),
            local_timestamp TIMESTAMP(3) WITH LOCAL TIME ZONE
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("type_boundaries").unwrap();

    for (column_name, expected_family, expected_native_type) in [
        (
            "number_unspecified",
            ColumnTypeFamily::Decimal,
            Some(KingbaseOracleType::Number(KingbaseOracleNumberArguments::Unspecified)),
        ),
        (
            "number_boolean",
            ColumnTypeFamily::Int,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(1, 0),
            )),
        ),
        (
            "float_value",
            ColumnTypeFamily::Float,
            Some(KingbaseOracleType::BinaryDouble),
        ),
        (
            "national_char",
            ColumnTypeFamily::String,
            // Kingbase Oracle exposes NCHAR as the same `bpchar` catalog type
            // as CHAR, so introspection uses the canonical character type.
            Some(KingbaseOracleType::Char(Some(10))),
        ),
        (
            "national_varchar",
            ColumnTypeFamily::String,
            // The catalog also normalizes NVARCHAR2 to `varchar`.
            Some(KingbaseOracleType::VarChar2(Some(32))),
        ),
        (
            "local_timestamp",
            ColumnTypeFamily::DateTime,
            // TIMESTAMP WITH LOCAL TIME ZONE is reported as PostgreSQL
            // `timestamp` by this Kingbase Oracle-mode catalog.
            Some(KingbaseOracleType::Timestamp(Some(3))),
        ),
    ] {
        let column = table.column(column_name).unwrap();
        assert_eq!(
            &expected_family,
            column.column_type_family(),
            "wrong type family for {column_name}"
        );
        let native_type = column
            .column_type()
            .native_type
            .as_ref()
            .map(|native_type| *native_type.downcast_ref::<KingbaseOracleType>());
        assert_eq!(expected_native_type, native_type, "wrong native type for {column_name}");
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_postgres_array_columns_remain_unsupported(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE unsupported_array_column (
            values INTEGER[]
        );
        "#,
    );

    let schema = api.describe();
    let column = schema
        .table_walker("unsupported_array_column")
        .unwrap()
        .column("values")
        .unwrap();

    assert!(matches!(column.column_type_family(), ColumnTypeFamily::Unsupported(_)));
    assert!(column.column_type().native_type.is_none());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_defaults_enums_and_foreign_keys_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TYPE order_status AS ENUM ('draft', 'published');

        CREATE TABLE parent (
            id NUMBER(10, 0) PRIMARY KEY
        );

        CREATE TABLE child (
            id SERIAL PRIMARY KEY,
            parent_id NUMBER(10, 0) NOT NULL,
            retries NUMBER(10, 0) NOT NULL DEFAULT 3,
            label VARCHAR2(32) NOT NULL DEFAULT 'pending',
            created_at TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
            status order_status NOT NULL,
            CONSTRAINT child_parent_fk
                FOREIGN KEY (parent_id)
                REFERENCES parent (id)
                ON DELETE CASCADE
        );
        "#,
    );

    let schema = api.describe();
    let child = schema.table_walker("child").unwrap();

    assert!(child.column("id").unwrap().is_autoincrement());
    assert_eq!(
        Some(&PrismaValue::Int(3)),
        child.column("retries").unwrap().default().unwrap().as_value()
    );
    assert_eq!(
        Some(&PrismaValue::String("pending".to_owned())),
        child.column("label").unwrap().default().unwrap().as_value()
    );
    assert!(child.column("created_at").unwrap().default().unwrap().is_now());

    let status = child.column("status").unwrap().column_type_family_as_enum().unwrap();
    assert_eq!("order_status", status.name());
    assert_eq!(vec!["draft", "published"], status.values().collect::<Vec<_>>());

    schema.assert_table("child", |table| {
        table.assert_foreign_key_on_columns(&["parent_id"], |foreign_key| {
            foreign_key
                .assert_references("parent", &["id"])
                .assert_on_delete(ForeignKeyAction::Cascade)
        })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_composite_foreign_keys_preserve_column_order(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE city (
            id NUMBER(10, 0) NOT NULL,
            code VARCHAR2(16) NOT NULL,
            PRIMARY KEY (id, code)
        );

        CREATE TABLE user_city (
            city_id NUMBER(10, 0) NOT NULL,
            city_code VARCHAR2(16) NOT NULL,
            CONSTRAINT user_city_city_fk
                FOREIGN KEY (city_id, city_code)
                REFERENCES city (id, code)
                ON DELETE CASCADE
        );
        "#,
    );

    api.describe().assert_table("user_city", |table| {
        table.assert_foreign_key_on_columns(&["city_id", "city_code"], |foreign_key| {
            foreign_key
                .assert_references("city", &["id", "code"])
                .assert_on_delete(ForeignKeyAction::Cascade)
        })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_check_constraints_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE products (
            id NUMBER(10, 0) NOT NULL PRIMARY KEY,
            price NUMBER(10, 0) NOT NULL,
            CONSTRAINT products_price_positive CHECK (price > 0)
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("products").unwrap();

    assert!(table.has_check_constraints());
    assert_eq!(
        vec!["products_price_positive"],
        table.check_constraints().collect::<Vec<_>>()
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_table_and_column_comments_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE comments (
            id NUMBER(10, 0) NOT NULL PRIMARY KEY
        );
        COMMENT ON TABLE comments IS 'commented table';
        COMMENT ON COLUMN comments.id IS 'identifier';
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("comments").unwrap();

    assert_eq!(Some("commented table"), table.description());
    assert_eq!(Some("identifier"), table.column("id").unwrap().description());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_views_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE a (a_id NUMBER(10, 0));
        CREATE TABLE b (b_id NUMBER(10, 0));
        CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
        "#,
    );

    let schema = api.describe();
    let view = schema.get_view("ab").expect("couldn't get ab view");

    assert_eq!("ab", &view.name);
    assert!(
        view.definition
            .as_deref()
            .is_some_and(|definition| definition.contains("UNION ALL"))
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_procedures_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE PROCEDURE describe_me (OUT result NUMBER)
        AS
        BEGIN
            SELECT 1 INTO result;
        END;
        "#,
    );

    let schema = api.describe();
    let procedure = schema
        .get_procedure("describe_me")
        .expect("couldn't get describe_me procedure");

    assert_eq!("describe_me", &procedure.name);
    assert!(procedure.definition.is_some());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_compound_unique_indexes_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE relation_index (
            left_id NUMBER(10, 0) NOT NULL,
            right_id NUMBER(10, 0) NOT NULL
        );
        CREATE UNIQUE INDEX relation_index_unique ON relation_index (left_id, right_id);
        "#,
    );

    api.describe().assert_table("relation_index", |table| {
        table.assert_index_on_columns(&["left_id", "right_id"], |index| {
            index.assert_name("relation_index_unique").assert_is_unique()
        })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_function_defaults_are_described_as_dbgenerated(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE expression_defaults (
            value NUMBER(10, 0) DEFAULT (ABS(8) + ABS(8))
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("expression_defaults").unwrap();

    assert!(table.column("value").unwrap().default().unwrap().is_db_generated());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_escaped_string_defaults_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE escaped_defaults (
            quote_value VARCHAR2(500) NOT NULL DEFAULT '"That''s a lot of fish!"\n - Godzilla, 1998',
            backslash_value VARCHAR2(255) NOT NULL DEFAULT 'xyz\Datasource\Model'
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("escaped_defaults").unwrap();

    assert_eq!(
        Some(&PrismaValue::String(
            "\"That's a lot of fish!\"\\n - Godzilla, 1998".to_owned()
        )),
        table.column("quote_value").unwrap().default().unwrap().as_value()
    );
    assert_eq!(
        Some(&PrismaValue::String("xyz\\Datasource\\Model".to_owned())),
        table.column("backslash_value").unwrap().default().unwrap().as_value()
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_constraints_outside_the_described_schema_are_ignored(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE SCHEMA other_schema;

        CREATE TABLE other_schema.constraint_parent (
            id NUMBER(10, 0) PRIMARY KEY
        );
        CREATE TABLE other_schema.constraint_child (
            id NUMBER(10, 0) PRIMARY KEY,
            parent_id NUMBER(10, 0),
            CONSTRAINT other_schema_parent_fk
                FOREIGN KEY (parent_id)
                REFERENCES other_schema.constraint_parent (id)
        );

        CREATE TABLE constraint_parent (
            id NUMBER(10, 0) PRIMARY KEY
        );
        CREATE TABLE constraint_child (
            id NUMBER(10, 0) PRIMARY KEY,
            parent_id NUMBER(10, 0),
            CONSTRAINT current_schema_parent_fk
                FOREIGN KEY (parent_id)
                REFERENCES constraint_parent (id)
        );
        "#,
    );

    let schema = api.describe();
    assert_eq!(1, schema.walk_foreign_keys().count());
    schema.assert_table("constraint_child", |table| {
        table.assert_foreign_key_on_columns(&["parent_id"], |foreign_key| {
            foreign_key.assert_references("constraint_parent", &["id"])
        })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_multiple_schemas_keep_same_named_tables_and_relations(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE SCHEMA one;
        CREATE SCHEMA two;

        CREATE TABLE one.parent (
            id NUMBER(10, 0) PRIMARY KEY
        );
        CREATE TABLE one.child (
            parent_id NUMBER(10, 0),
            CONSTRAINT one_child_parent_fk
                FOREIGN KEY (parent_id) REFERENCES one.parent (id)
        );

        CREATE TABLE two.parent (
            id NUMBER(10, 0) PRIMARY KEY
        );
        CREATE TABLE two.child (
            parent_id NUMBER(10, 0),
            CONSTRAINT two_child_parent_fk
                FOREIGN KEY (parent_id) REFERENCES two.parent (id)
        );
        "#,
    );

    let schema = api.describe_with_schemas(&["one", "two"]);
    schema.assert_namespace("one").assert_namespace("two");
    assert_eq!(4, schema.table_walkers().len());
    assert_eq!(2, schema.walk_foreign_keys().count());

    for namespace in ["one", "two"] {
        let child = schema
            .table_walkers()
            .find(|table| table.name() == "child" && table.namespace() == Some(namespace))
            .unwrap();
        let foreign_key = child.foreign_keys().next().unwrap();

        assert_eq!("parent", foreign_key.referenced_table_name());
        assert_eq!(Some(namespace), foreign_key.referenced_table().namespace());
    }
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_foreign_key_delete_actions_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE city (
            id NUMBER(10, 0) PRIMARY KEY
        );
        CREATE TABLE user_city (
            id NUMBER(10, 0) PRIMARY KEY,
            city_no_action NUMBER(10, 0),
            city_cascade NUMBER(10, 0),
            city_restrict NUMBER(10, 0),
            city_set_null NUMBER(10, 0),
            city_set_default NUMBER(10, 0) DEFAULT 1,
            FOREIGN KEY (city_no_action) REFERENCES city (id) ON DELETE NO ACTION,
            FOREIGN KEY (city_cascade) REFERENCES city (id) ON DELETE CASCADE,
            FOREIGN KEY (city_restrict) REFERENCES city (id) ON DELETE RESTRICT,
            FOREIGN KEY (city_set_null) REFERENCES city (id) ON DELETE SET NULL,
            FOREIGN KEY (city_set_default) REFERENCES city (id) ON DELETE SET DEFAULT
        );
        "#,
    );

    api.describe().assert_table("user_city", |table| {
        table
            .assert_foreign_key_on_columns(&["city_no_action"], |foreign_key| {
                foreign_key
                    .assert_references("city", &["id"])
                    .assert_on_delete(ForeignKeyAction::NoAction)
            })
            .assert_foreign_key_on_columns(&["city_cascade"], |foreign_key| {
                foreign_key
                    .assert_references("city", &["id"])
                    .assert_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_foreign_key_on_columns(&["city_restrict"], |foreign_key| {
                foreign_key
                    .assert_references("city", &["id"])
                    .assert_on_delete(ForeignKeyAction::Restrict)
            })
            .assert_foreign_key_on_columns(&["city_set_null"], |foreign_key| {
                foreign_key
                    .assert_references("city", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetNull)
            })
            .assert_foreign_key_on_columns(&["city_set_default"], |foreign_key| {
                foreign_key
                    .assert_references("city", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetDefault)
            })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_mode_composite_index_sort_order_is_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE sorted_values (
            first_value NUMBER(10, 0) NOT NULL,
            second_value NUMBER(10, 0) NOT NULL
        );

        CREATE INDEX sorted_values_index ON sorted_values (first_value ASC, second_value DESC);
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("sorted_values").unwrap();
    let index = table
        .indexes()
        .find(|index| index.name() == "sorted_values_index")
        .unwrap();
    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());
    assert_eq!("first_value", columns[0].as_column().name());
    assert_eq!("second_value", columns[1].as_column().name());
    assert_eq!(Some(SQLSortOrder::Asc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_independent_dictionary_namespaces_defaults_and_actions(api: TestApi) {
    let setup = r#"
        CREATE SCHEMA independent_one;
        CREATE SCHEMA independent_two;
        CREATE SEQUENCE independent_one."ExternalSeq";
        CREATE TABLE independent_one."SameTable"(
            id NUMBER(10,0) DEFAULT nextval('independent_one."ExternalSeq"'),
            value NUMBER(3,3) DEFAULT NULLIF(0.012,0.013),
            label VARCHAR2(60) DEFAULT q'[it's :ignored]',
            payload BLOB DEFAULT decoding('00abcd','hex'),
            CONSTRAINT same_pk PRIMARY KEY(id), CONSTRAINT same_unique UNIQUE(label));
        CREATE TABLE independent_two."SameTable"(
            id NUMBER(10,0), parent_id NUMBER(10,0), label VARCHAR2(60),
            CONSTRAINT same_pk PRIMARY KEY(id),
            CONSTRAINT parent_fk FOREIGN KEY(parent_id) REFERENCES independent_one."SameTable"(id) ON DELETE SET NULL ON UPDATE CASCADE);
        CREATE INDEX same_index ON independent_two."SameTable"(label DESC);
        CREATE TABLE independent_two.unquoted_lower(id NUMBER(10,0));
        COMMENT ON COLUMN independent_one."SameTable".label IS 'exact metadata';
    "#;
    for statement in setup.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        tok(api.database().raw_cmd(statement)).unwrap_or_else(|error| panic!("{error}\nSQL: {statement}"));
    }
    let schema = api.describe_with_schemas(&["independent_one", "independent_two"]);
    let first = schema.table_walker_ns("independent_one", "SameTable").unwrap();
    let second = schema.table_walker_ns("independent_two", "SameTable").unwrap();
    assert!(schema.table_walker_ns("independent_two", "unquoted_lower").is_some());
    assert!(first.column("id").unwrap().is_autoincrement());
    let label = first.column("label").unwrap();
    assert_eq!(label.description(), Some("exact metadata"));
    assert_eq!(
        label.default().unwrap().kind(),
        &sql_schema_describer::DefaultKind::Value(PrismaValue::String("it's :ignored".into()))
    );
    assert!(matches!(
        first.column("value").unwrap().default().unwrap().kind(),
        sql_schema_describer::DefaultKind::DbGenerated(_)
    ));
    assert_eq!(
        first.column("payload").unwrap().default().unwrap().kind(),
        &sql_schema_describer::DefaultKind::Value(PrismaValue::Bytes(vec![0, 0xab, 0xcd]))
    );
    let fk = second.foreign_keys().next().unwrap();
    assert_eq!(fk.referenced_table().id, first.id);
    assert_eq!(fk.on_delete_action(), ForeignKeyAction::SetNull);
    assert_eq!(fk.on_update_action(), ForeignKeyAction::Cascade);
    assert_eq!(
        second
            .indexes()
            .find(|i| i.name() == "same_index")
            .unwrap()
            .columns()
            .next()
            .unwrap()
            .sort_order(),
        Some(SQLSortOrder::Desc)
    );
    let ext: &sql_schema_describer::kingbase_oracle::KingbaseOracleSchemaExt = schema.downcast_connector_data();
    assert_eq!(
        ext.owning_constraint_names
            .get(&first.primary_key().unwrap().id)
            .map(String::as_str),
        Some("same_pk")
    );
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_include_keys_cast_defaults_and_generation_ownership(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE "Covered" (a NUMBER(10,0), b NUMBER(10,0));
        CREATE UNIQUE INDEX covered_uq ON "Covered"(a) INCLUDE(b);
        CREATE TABLE "Casted" (n NUMBER(10,2) DEFAULT CAST(1.25 AS NUMERIC(2,1)));
        INSERT INTO "Casted" DEFAULT VALUES;
        CREATE SEQUENCE external_seq;
        CREATE SEQUENCE shared_seq;
        CREATE TABLE "Owned" (id INTEGER DEFAULT nextval('shared_seq'));
        ALTER SEQUENCE shared_seq OWNED BY "Owned".id;
        CREATE TABLE "External" (id INTEGER DEFAULT nextval('external_seq'));
        CREATE TABLE "Shared" (id INTEGER DEFAULT nextval('shared_seq'));
        CREATE TABLE "NativeIdentity" (id INTEGER GENERATED BY DEFAULT AS IDENTITY);
    "#,
    );
    let schema = api.describe();
    let covered = schema.table_walker("Covered").unwrap();
    let index = covered.indexes().find(|i| i.name() == "covered_uq").unwrap();
    assert!(index.is_unique());
    assert_eq!(index.column_names().collect::<Vec<_>>(), vec!["a"]);
    let default = schema
        .table_walker("Casted")
        .unwrap()
        .column("n")
        .unwrap()
        .default()
        .unwrap();
    assert!(
        matches!(default.kind(), sql_schema_describer::DefaultKind::DbGenerated(Some(expression)) if expression.contains("numeric(2,1)"))
    );
    let actual = tok(api
        .database()
        .query_raw("SELECT CAST(n AS TEXT) AS n FROM \"Casted\"", &[]))
    .unwrap();
    assert_eq!(actual.into_single().unwrap().get("n").unwrap().as_str(), Some("1.30"));
    use sql_schema_describer::kingbase_oracle::{ColumnGeneration, KingbaseOracleSchemaExt};
    let ext: &KingbaseOracleSchemaExt = schema.downcast_connector_data();
    let column = |table| schema.table_walker(table).unwrap().column("id").unwrap().id;
    assert!(!ext.column_generations.contains_key(&column("External")));
    assert!(!ext.column_generations.contains_key(&column("Shared")));
    assert!(
        matches!(ext.column_generations.get(&column("Owned")),Some(ColumnGeneration::OwnedSequence{name,shared:true,..}) if name=="shared_seq")
    );
    assert!(matches!(
        ext.column_generations.get(&column("NativeIdentity")),
        Some(ColumnGeneration::Identity { .. })
    ));
}
