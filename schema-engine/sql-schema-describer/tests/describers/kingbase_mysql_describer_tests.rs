use crate::test_api::*;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use psl::builtin_connectors::KingbaseMySqlType;
use sql_schema_describer::ColumnTypeFamily;
use sql_schema_describer::ForeignKeyAction;

#[test_connector(tags(KingbaseMysql))]
fn regression_non_literal_string_defaults_remain_expressions(api: TestApi) {
    api.raw_cmd(
        "CREATE TABLE expression_defaults(id INT PRIMARY KEY,
        upper_value VARCHAR(50) NOT NULL DEFAULT upper('abc'),
        lower_value VARCHAR(50) NOT NULL DEFAULT lower('ABC'),
        missing_value VARCHAR(50) DEFAULT NULLIF('a','a'),
        literal_value VARCHAR(50) NOT NULL DEFAULT 'upper(''abc'')')",
    );
    let schema = api.describe();
    let table = schema.table_walker("expression_defaults").unwrap();
    let expressions = api.block_on(api.database().query_raw(
        "SELECT column_name,column_default FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='expression_defaults'", &[])).unwrap();
    for name in ["upper_value", "lower_value", "missing_value"] {
        let expression = expressions
            .iter()
            .find(|row| row.get("column_name").unwrap().as_str() == Some(name))
            .unwrap();
        assert_eq!(
            table.column(name).unwrap().default().unwrap().kind(),
            &sql_schema_describer::DefaultKind::DbGenerated(Some(
                expression.get("column_default").unwrap().as_str().unwrap().to_owned()
            )),
            "{name}"
        );
    }
    assert_eq!(
        table.column("literal_value").unwrap().default().unwrap().as_value(),
        Some(&PrismaValue::String("upper('abc')".into()))
    );
    api.raw_cmd("INSERT INTO expression_defaults(id) VALUES(1)");
    let rows = api
        .block_on(api.database().query_raw(
            "SELECT upper_value,lower_value,missing_value,literal_value FROM expression_defaults",
            &[],
        ))
        .unwrap();
    let row = rows.into_iter().next().unwrap();
    assert_eq!(row.get("upper_value").unwrap().as_str(), Some("ABC"));
    assert_eq!(row.get("lower_value").unwrap().as_str(), Some("abc"));
    assert!(row.get("missing_value").unwrap().is_null());
    assert_eq!(row.get("literal_value").unwrap().as_str(), Some("upper('abc')"));
}

#[test_connector(tags(KingbaseMysql))]
fn regression_string_and_enum_default_literals_are_decoded_once(api: TestApi) {
    let cases = [
        ("single_quote", "O'Reilly"),
        ("two_quotes", "a''b"),
        ("three_quotes", "a'''b"),
        ("quote_edges", "''"),
        ("one_slash", r"C:\path"),
        ("two_slashes", r"C:\\path"),
        ("slash_quote", r"C:\'path"),
        ("charset_text", "_utf8mb4"),
        ("colons", "ab::cd"),
        ("empty", ""),
        ("linefeed", "line\nnext"),
        ("unicode", "字"),
        ("plain", "pending"),
    ];
    let mut ddl = "CREATE TABLE literal_defaults(id INT PRIMARY KEY".to_owned();
    for (name, value) in cases {
        let literal = format!("'{}'", value.replace('\'', "''"));
        ddl.push_str(&format!(",s_{name} VARCHAR(100) NOT NULL DEFAULT {literal},e_{name} ENUM({literal},'other') NOT NULL DEFAULT {literal}"));
    }
    ddl.push(')');
    api.raw_cmd(&ddl);
    let schema = api.describe();
    let table = schema.table_walker("literal_defaults").unwrap();
    api.raw_cmd("INSERT INTO literal_defaults(id) VALUES(1)");
    let selected_columns = cases
        .iter()
        .flat_map(|(name, _)| ["s", "e"].map(|prefix| format!("CAST({prefix}_{name} AS TEXT) AS {prefix}_{name}")))
        .collect::<Vec<_>>()
        .join(",");
    let rows = api
        .block_on(
            api.database()
                .query_raw(&format!("SELECT {selected_columns} FROM literal_defaults"), &[]),
        )
        .unwrap();
    let row = rows.into_iter().next().unwrap();
    for (name, value) in cases {
        for (prefix, expected) in [
            ("s", PrismaValue::String(value.into())),
            ("e", PrismaValue::Enum(value.into())),
        ] {
            let name = format!("{prefix}_{name}");
            assert_eq!(row.get(&name).unwrap().as_str(), Some(value), "actual server {name}");
            assert_eq!(
                table.column(&name).unwrap().default().unwrap().as_value(),
                Some(&expected),
                "introspected {name}"
            );
        }
    }
}

#[test_connector(tags(KingbaseMysql))]
fn regression_include_columns_are_not_index_keys(api: TestApi) {
    use sql_schema_describer::SQLSortOrder::{Asc, Desc};
    api.raw_cmd(
        "CREATE TABLE included(id INT PRIMARY KEY,a INT NOT NULL,b INT NOT NULL,payload VARCHAR(32));
        CREATE UNIQUE INDEX included_unique ON included(a) INCLUDE(b,payload);
        CREATE INDEX included_regular ON included(b DESC,a ASC) INCLUDE(payload)",
    );
    let schema = api.describe();
    let table = schema.table_walker("included").unwrap();
    assert_eq!(
        table.columns().map(|column| column.name()).collect::<Vec<_>>(),
        vec!["id", "a", "b", "payload"]
    );
    let unique = table.indexes().find(|index| index.name() == "included_unique").unwrap();
    assert!(unique.is_unique());
    assert_eq!(
        unique
            .columns()
            .map(|column| column.as_column().name())
            .collect::<Vec<_>>(),
        vec!["a"]
    );
    let regular = table
        .indexes()
        .find(|index| index.name() == "included_regular")
        .unwrap();
    assert!(!regular.is_unique());
    assert_eq!(
        regular
            .columns()
            .map(|column| (column.as_column().name(), column.sort_order()))
            .collect::<Vec<_>>(),
        vec![("b", Some(Desc)), ("a", Some(Asc))]
    );
    assert_eq!(
        table
            .primary_key_columns()
            .unwrap()
            .map(|column| column.as_column().name())
            .collect::<Vec<_>>(),
        vec!["id"]
    );
}

#[test_connector(tags(KingbaseMysql))]
fn regression_string_cast_defaults_remain_expressions(api: TestApi) {
    api.raw_cmd(
        "CREATE TABLE cast_defaults(id INT PRIMARY KEY,
        truncated VARCHAR(30) NOT NULL DEFAULT CAST('abcdef' AS CHAR(3)),
        quoted VARCHAR(30) NOT NULL DEFAULT CAST('O''Reilly' AS CHAR(4)),
        plain VARCHAR(30) NOT NULL DEFAULT 'abcdef',
        colons VARCHAR(30) NOT NULL DEFAULT 'ab::cd',
        trimmed VARCHAR(30) NOT NULL DEFAULT CAST('abc ' AS bpchar),
        fixed CHAR(5) NOT NULL DEFAULT 'abc')",
    );
    let schema = api.describe();
    let table = schema.table_walker("cast_defaults").unwrap();
    for name in ["truncated", "quoted", "trimmed"] {
        assert!(
            table.column(name).unwrap().default().unwrap().is_db_generated(),
            "{name}"
        );
    }
    for (name, value) in [("plain", "abcdef"), ("colons", "ab::cd"), ("fixed", "abc")] {
        assert_eq!(
            table.column(name).unwrap().default().unwrap().as_value(),
            Some(&PrismaValue::String(value.into()))
        );
    }
    api.raw_cmd("INSERT INTO cast_defaults(id) VALUES(1)");
    let rows = api
        .block_on(
            api.database()
                .query_raw("SELECT truncated,quoted,plain,colons,trimmed FROM cast_defaults", &[]),
        )
        .unwrap();
    let row = rows.into_iter().next().unwrap();
    for (name, value) in [
        ("truncated", "abc"),
        ("trimmed", "abc"),
        ("quoted", "O'Re"),
        ("plain", "abcdef"),
        ("colons", "ab::cd"),
    ] {
        assert_eq!(row.get(name).unwrap().as_str(), Some(value));
    }
}

#[test_connector(tags(KingbaseMysql))]
fn index_column_sort_orders_are_described(api: TestApi) {
    use sql_schema_describer::SQLSortOrder::{Asc, Desc};

    api.raw_cmd(
        "CREATE TABLE sort_orders(id INT PRIMARY KEY, a INT NOT NULL, b INT NOT NULL); \
         CREATE INDEX mixed_order ON sort_orders(a ASC, b DESC); \
         CREATE UNIQUE INDEX reverse_order ON sort_orders(a DESC, b ASC); \
         CREATE INDEX hash_order ON sort_orders USING hash(a);",
    );
    let schema = api.describe();
    let table = schema
        .table_walkers()
        .find(|table| table.name() == "sort_orders")
        .unwrap();
    for (name, expected) in [
        ("mixed_order", vec![Some(Asc), Some(Desc)]),
        ("reverse_order", vec![Some(Desc), Some(Asc)]),
        ("hash_order", vec![None]),
    ] {
        let index = table.indexes().find(|index| index.name() == name).unwrap();
        assert_eq!(
            index.columns().map(|column| column.sort_order()).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test_connector(tags(KingbaseMysql))]
fn all_kingbase_mysql_column_types_must_work(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE type_samples (
            id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
            int_value INT,
            smallint_value SMALLINT,
            tinyint_value TINYINT,
            tinyint_bool TINYINT(1),
            mediumint_value MEDIUMINT,
            bigint_value BIGINT,
            decimal_value DECIMAL(5, 3),
            numeric_value NUMERIC(4, 1),
            float_value FLOAT,
            double_value DOUBLE,
            bit_bool BIT(1),
            bit_value BIT(8),
            char_value CHAR(10),
            varchar_value VARCHAR(32),
            binary_value BINARY(8),
            varbinary_value VARBINARY(8),
            tiny_blob TINYBLOB,
            blob_value BLOB,
            medium_blob MEDIUMBLOB,
            long_blob LONGBLOB,
            tiny_text TINYTEXT,
            text_value TEXT,
            medium_text MEDIUMTEXT,
            long_text LONGTEXT,
            date_value DATE,
            time_value TIME(3),
            datetime_value DATETIME(3),
            timestamp_value TIMESTAMP(3),
            year_value YEAR,
            json_value JSON,
            status ENUM('draft', 'published'),
            labels SET('red', 'blue')
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("type_samples").unwrap();

    assert_eq!(1, table.primary_key_columns_count());
    assert!(table.column("id").unwrap().is_autoincrement());

    for (column_name, expected_family, expected_native_type) in [
        ("id", ColumnTypeFamily::Int, Some(KingbaseMySqlType::Int)),
        ("int_value", ColumnTypeFamily::Int, Some(KingbaseMySqlType::Int)),
        (
            "smallint_value",
            ColumnTypeFamily::Int,
            Some(KingbaseMySqlType::SmallInt),
        ),
        ("tinyint_value", ColumnTypeFamily::Int, Some(KingbaseMySqlType::TinyInt)),
        ("tinyint_bool", ColumnTypeFamily::Boolean, None),
        (
            "mediumint_value",
            ColumnTypeFamily::Int,
            Some(KingbaseMySqlType::MediumInt),
        ),
        (
            "bigint_value",
            ColumnTypeFamily::BigInt,
            Some(KingbaseMySqlType::BigInt),
        ),
        (
            "decimal_value",
            ColumnTypeFamily::Decimal,
            Some(KingbaseMySqlType::Decimal(Some((5, 3)))),
        ),
        (
            "numeric_value",
            ColumnTypeFamily::Decimal,
            Some(KingbaseMySqlType::Decimal(Some((4, 1)))),
        ),
        ("float_value", ColumnTypeFamily::Float, Some(KingbaseMySqlType::Float)),
        ("double_value", ColumnTypeFamily::Float, Some(KingbaseMySqlType::Double)),
        ("bit_bool", ColumnTypeFamily::Boolean, Some(KingbaseMySqlType::Bit(1))),
        ("bit_value", ColumnTypeFamily::Binary, Some(KingbaseMySqlType::Bit(8))),
        (
            "char_value",
            ColumnTypeFamily::String,
            Some(KingbaseMySqlType::Char(10)),
        ),
        (
            "varchar_value",
            ColumnTypeFamily::String,
            Some(KingbaseMySqlType::VarChar(32)),
        ),
        (
            "binary_value",
            ColumnTypeFamily::Binary,
            Some(KingbaseMySqlType::Binary(8)),
        ),
        (
            "varbinary_value",
            ColumnTypeFamily::Binary,
            Some(KingbaseMySqlType::VarBinary(8)),
        ),
        ("tiny_blob", ColumnTypeFamily::Binary, Some(KingbaseMySqlType::TinyBlob)),
        ("blob_value", ColumnTypeFamily::Binary, Some(KingbaseMySqlType::Blob)),
        (
            "medium_blob",
            ColumnTypeFamily::Binary,
            Some(KingbaseMySqlType::MediumBlob),
        ),
        ("long_blob", ColumnTypeFamily::Binary, Some(KingbaseMySqlType::LongBlob)),
        ("tiny_text", ColumnTypeFamily::String, Some(KingbaseMySqlType::TinyText)),
        ("text_value", ColumnTypeFamily::String, Some(KingbaseMySqlType::Text)),
        (
            "medium_text",
            ColumnTypeFamily::String,
            Some(KingbaseMySqlType::MediumText),
        ),
        ("long_text", ColumnTypeFamily::String, Some(KingbaseMySqlType::LongText)),
        ("date_value", ColumnTypeFamily::DateTime, Some(KingbaseMySqlType::Date)),
        (
            "time_value",
            ColumnTypeFamily::DateTime,
            Some(KingbaseMySqlType::Time(Some(3))),
        ),
        (
            "datetime_value",
            ColumnTypeFamily::DateTime,
            Some(KingbaseMySqlType::DateTime(Some(3))),
        ),
        (
            "timestamp_value",
            ColumnTypeFamily::DateTime,
            Some(KingbaseMySqlType::Timestamp(Some(3))),
        ),
        ("year_value", ColumnTypeFamily::Int, Some(KingbaseMySqlType::Year)),
        ("json_value", ColumnTypeFamily::Json, Some(KingbaseMySqlType::Json)),
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
            .map(|native_type| *native_type.downcast_ref::<KingbaseMySqlType>());

        assert_eq!(expected_native_type, native_type, "wrong native type for {column_name}");
    }

    let status = table.column("status").unwrap();
    assert!(status.column_type_family().is_enum());
    let status_enum = status.column_type_family_as_enum().unwrap();
    assert_eq!("type_samples_status", status_enum.name());
    assert_eq!(vec!["draft", "published"], status_enum.values().collect::<Vec<_>>());

    assert_eq!(
        &ColumnTypeFamily::String,
        table.column("labels").unwrap().column_type_family()
    );
}

#[test_connector(tags(KingbaseMysql))]
fn multi_column_foreign_keys_are_described_in_column_order(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE City (
            id INT NOT NULL,
            code VARCHAR(16) NOT NULL,
            PRIMARY KEY (id, code)
        );

        CREATE TABLE UserCity (
            city_id INT NOT NULL,
            city_code VARCHAR(16) NOT NULL,
            CONSTRAINT UserCity_city_fkey
                FOREIGN KEY (city_id, city_code)
                REFERENCES City (id, code)
                ON DELETE CASCADE
        );
        "#,
    );

    api.describe().assert_table("UserCity", |table| {
        table.assert_foreign_key_on_columns(&["city_id", "city_code"], |foreign_key| {
            foreign_key
                .assert_references("City", &["id", "code"])
                .assert_on_delete(ForeignKeyAction::Cascade)
        })
    });
}

#[test_connector(tags(KingbaseMysql))]
fn defaults_and_auto_increment_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE defaults (
            id INT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            retries INT NOT NULL DEFAULT 3,
            label VARCHAR(32) NOT NULL DEFAULT 'pending',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("defaults").unwrap();

    assert!(table.column("id").unwrap().is_autoincrement());
    assert_eq!(
        Some(&PrismaValue::Int(3)),
        table.column("retries").unwrap().default().unwrap().as_value()
    );
    assert_eq!(
        Some(&PrismaValue::String("pending".to_owned())),
        table.column("label").unwrap().default().unwrap().as_value()
    );
    assert!(table.column("created_at").unwrap().default().unwrap().is_now());
}

#[test_connector(tags(KingbaseMysql))]
fn escaped_string_defaults_are_described(api: TestApi) {
    let sql = r#"
        CREATE TABLE escaped_defaults (
            quote_value VARCHAR(500) NOT NULL DEFAULT '"That''s a lot of fish!"\n - Godzilla, 1998',
            backslash_value VARCHAR(255) NOT NULL DEFAULT 'xyz\\Datasource\\Model'
        );
    "#;

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walker("escaped_defaults").unwrap();
    let expected_quote = "\"That's a lot of fish!\"\\n - Godzilla, 1998";
    let expected_backslashes = r"xyz\\Datasource\\Model";

    api.raw_cmd("INSERT INTO escaped_defaults DEFAULT VALUES");
    let rows = api
        .block_on(
            api.database()
                .query_raw("SELECT quote_value,backslash_value FROM escaped_defaults", &[]),
        )
        .unwrap();
    let row = rows.into_iter().next().unwrap();
    assert_eq!(row.get("quote_value").unwrap().as_str(), Some(expected_quote));
    assert_eq!(row.get("backslash_value").unwrap().as_str(), Some(expected_backslashes));

    assert_eq!(
        Some(&PrismaValue::String(expected_quote.to_owned())),
        table.column("quote_value").unwrap().default().unwrap().as_value()
    );
    assert_eq!(
        Some(&PrismaValue::String(expected_backslashes.to_owned())),
        table.column("backslash_value").unwrap().default().unwrap().as_value()
    );
}

#[test_connector(tags(KingbaseMysql))]
fn function_expression_defaults_are_described_as_dbgenerated(api: TestApi) {
    let sql = r#"
        CREATE TABLE expression_defaults (
            int_value INT DEFAULT (ABS(8) + ABS(8))
        );
    "#;

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walker("expression_defaults").unwrap();

    assert!(table.column("int_value").unwrap().default().unwrap().is_db_generated());
}

#[test_connector(tags(KingbaseMysql))]
fn constraints_from_other_schemas_are_not_described(api: TestApi) {
    let sql = r#"
        DROP SCHEMA IF EXISTS other_schema CASCADE;
        CREATE SCHEMA other_schema;

        CREATE TABLE other_schema.constraint_parent (id INT PRIMARY KEY);
        CREATE TABLE other_schema.constraint_child (
            id INT PRIMARY KEY,
            parent_id INT,
            CONSTRAINT shared_parent_fk FOREIGN KEY (parent_id)
                REFERENCES other_schema.constraint_parent (id)
        );

        CREATE TABLE constraint_parent (id INT PRIMARY KEY);
        CREATE TABLE constraint_child (
            id INT PRIMARY KEY,
            parent_id INT,
            CONSTRAINT shared_parent_fk FOREIGN KEY (parent_id)
                REFERENCES constraint_parent (id)
        );
        "#;

    api.raw_cmd(sql);

    let schema = api.describe();
    assert_eq!(1, schema.walk_foreign_keys().count());
    schema.assert_table("constraint_child", |table| {
        table.assert_foreign_key_on_columns(&["parent_id"], |foreign_key| {
            foreign_key.assert_references("constraint_parent", &["id"])
        })
    });
}

#[test_connector(tags(KingbaseMysql))]
fn check_constraints_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE products (
            id INT NOT NULL PRIMARY KEY,
            price INT NOT NULL,
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

#[test_connector(tags(KingbaseMysql))]
fn table_and_column_comments_are_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE comments (
            id INT NOT NULL COMMENT 'identifier'
        ) COMMENT = 'commented table';
        "#,
    );

    let schema = api.describe();
    let table = schema.table_walker("comments").unwrap();

    assert_eq!(Some("commented table"), table.description());
    assert_eq!(Some("identifier"), table.column("id").unwrap().description());
}

#[test_connector(tags(KingbaseMysql))]
fn views_can_be_described(api: TestApi) {
    let sql = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
        CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
    "#;

    api.raw_cmd(sql);

    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    let expected_sql = " SELECT a.a_id\n   FROM a\nUNION ALL\n SELECT b.b_id AS a_id\n   FROM b;";

    assert_eq!("ab", &view.name);
    assert_eq!(expected_sql, view.definition.unwrap());
}

#[test_connector(tags(KingbaseMysql))]
fn extension_owned_views_are_not_described(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE view_source (id INT);
        CREATE VIEW user_view AS SELECT id FROM view_source;
        CREATE VIEW extension_view AS SELECT id FROM view_source;
        ALTER EXTENSION plpgsql ADD VIEW extension_view;
        "#,
    );

    let schema = api.describe();

    assert!(schema.get_view("user_view").is_some());
    assert!(schema.get_view("extension_view").is_none());
}

#[test_connector(tags(KingbaseMysql))]
fn procedures_can_be_described(api: TestApi) {
    let sql = format!(
        r#"
        CREATE PROCEDURE {}.foo (OUT res INT) SELECT 1 INTO res
        "#,
        api.schema_name()
    );

    api.raw_cmd(&sql);
    let result = api.describe();
    let procedure = result
        .get_procedure("foo")
        .expect("couldn't get foo procedure")
        .to_owned();

    assert_eq!("foo", &procedure.name);
    assert_eq!(
        Some("begin\nSELECT 1 INTO res\n        \n;end"),
        procedure.definition.as_deref()
    );
}

#[test_connector(tags(KingbaseMysql))]
fn foreign_key_on_delete_must_be_handled(api: TestApi) {
    // Mirrors `mysql_foreign_key_on_delete_must_be_handled`. Kingbase uses a schema as
    // its object namespace, while MySQL uses the current database.
    let sql = format!(
        "CREATE TABLE `{0}`.City (id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY);
         CREATE TABLE `{0}`.User (
            id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
            city INTEGER, FOREIGN KEY(city) REFERENCES City (id) ON DELETE NO ACTION,
            city_cascade INTEGER, FOREIGN KEY(city_cascade) REFERENCES City (id) ON DELETE CASCADE,
            city_restrict INTEGER, FOREIGN KEY(city_restrict) REFERENCES City (id) ON DELETE RESTRICT,
            city_set_null INTEGER, FOREIGN KEY(city_set_null) REFERENCES City (id) ON DELETE SET NULL
        )",
        api.schema_name()
    );
    api.raw_cmd(&sql);

    api.describe().assert_table("User", |table| {
        // Kingbase does not automatically create an index for foreign key columns.
        table
            .assert_column("id", |id| id.assert_type_is_int())
            .assert_column("city", |column| column.assert_type_is_int())
            .assert_column("city_cascade", |column| column.assert_type_is_int())
            .assert_column("city_restrict", |column| column.assert_type_is_int())
            .assert_column("city_set_null", |column| column.assert_type_is_int())
            .assert_foreign_key_on_columns(&["city"], |foreign_key| {
                foreign_key
                    .assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::NoAction)
            })
            .assert_foreign_key_on_columns(&["city_cascade"], |foreign_key| {
                foreign_key
                    .assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_foreign_key_on_columns(&["city_restrict"], |foreign_key| {
                foreign_key
                    .assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Restrict)
            })
            .assert_foreign_key_on_columns(&["city_set_null"], |foreign_key| {
                foreign_key
                    .assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetNull)
            })
    });
}

#[test_connector(tags(KingbaseMysql))]
fn join_table_unique_indexes_must_be_inferred(api: TestApi) {
    // Mirrors `mysql_join_table_unique_indexes_must_be_inferred`.
    let sql = r#"
        CREATE TABLE `Cat` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            name TEXT
        );

        CREATE TABLE `Human` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            name TEXT
        );

        CREATE TABLE `CatToHuman` (
            cat INTEGER REFERENCES `Cat`(id),
            human INTEGER REFERENCES `Human`(id),
            relationship TEXT
        );

        CREATE UNIQUE INDEX cat_and_human_index ON `CatToHuman`(cat, human);
    "#;
    api.raw_cmd(sql);

    api.describe().assert_table("CatToHuman", |table| {
        table.assert_index_on_columns(&["cat", "human"], |index| {
            index.assert_name("cat_and_human_index").assert_is_unique()
        })
    });
}

#[test_connector(tags(KingbaseMysql))]
fn actual_primary_key_constraint_names_are_described(api: TestApi) {
    use sql_schema_describer::kingbase_mysql::KingbaseMysqlSchemaExt;

    api.raw_cmd(
        r#"
        CREATE TABLE default_pk_table(id INT NOT NULL PRIMARY KEY);
        CREATE TABLE custom_pk_table(id INT NOT NULL, CONSTRAINT custom_primary_key PRIMARY KEY(id));
        CREATE TABLE original_pk_table(id INT NOT NULL PRIMARY KEY);
        ALTER TABLE original_pk_table RENAME TO renamed_pk_table;
        "#,
    );

    let schema = api.describe();
    let ext: &KingbaseMysqlSchemaExt = schema.downcast_connector_data();
    assert_eq!(ext.primary_key_constraint_names.len(), 3);
    assert!(ext.unique_constraint_names.is_empty());

    for (table_name, constraint_name) in [
        ("default_pk_table", "default_pk_table_pkey"),
        ("custom_pk_table", "custom_primary_key"),
        ("renamed_pk_table", "original_pk_table_pkey"),
    ] {
        let primary_key = schema
            .table_walkers()
            .find(|table| table.name() == table_name)
            .unwrap()
            .primary_key()
            .unwrap();
        // Preserve MySQL-compatible logical metadata while retaining the
        // physical constraint name separately for migration rendering.
        assert_eq!(primary_key.name(), "");
        assert_eq!(
            ext.primary_key_constraint_names
                .get(&primary_key.id)
                .map(String::as_str),
            Some(constraint_name),
        );
    }
}

#[test_connector(tags(KingbaseMysql))]
fn unique_index_constraint_ownership_is_described(api: TestApi) {
    use sql_schema_describer::kingbase_mysql::KingbaseMysqlSchemaExt;

    api.raw_cmd(
        r#"
        CREATE TABLE index_owners (
            id INT NOT NULL PRIMARY KEY,
            a INT NOT NULL,
            b INT NOT NULL,
            c INT NOT NULL,
            UNIQUE INDEX inline_unique(a),
            UNIQUE INDEX compound_unique(a, b)
        );
        CREATE UNIQUE INDEX standalone_unique ON index_owners(b, c);
        CREATE INDEX ordinary_index ON index_owners(c);
        CREATE TABLE index_reference (
            id INT NOT NULL PRIMARY KEY,
            owner_a INT REFERENCES index_owners(a)
        );
        "#,
    );

    let schema = api.describe();
    let ext: &KingbaseMysqlSchemaExt = schema.downcast_connector_data();
    let table = schema
        .table_walkers()
        .find(|table| table.name() == "index_owners")
        .unwrap();
    assert_eq!(table.indexes().count(), 5);
    assert_eq!(ext.unique_constraint_names.len(), 2);

    for index in table.indexes() {
        let constraint_name = ext.unique_constraint_names.get(&index.id).map(String::as_str);
        match index.name() {
            "inline_unique" | "compound_unique" => assert_eq!(constraint_name, Some(index.name())),
            _ => assert_eq!(constraint_name, None),
        }
    }
}

#[test_connector(tags(KingbaseMysql))]
fn expression_indexes_do_not_corrupt_following_indexes(api: TestApi) {
    api.raw_cmd(
        r#"
        CREATE TABLE expression_indexes (
            id INT NOT NULL PRIMARY KEY,
            name VARCHAR(64) NOT NULL,
            age INT NOT NULL
        );

        CREATE INDEX expression_first ON expression_indexes ((lower(name)), age);
        CREATE INDEX ordinary_index ON expression_indexes (age);
        "#,
    );

    api.describe().assert_table("expression_indexes", |table| {
        table.assert_index_on_columns(&["age"], |index| index.assert_name("ordinary_index"))
    });
}
