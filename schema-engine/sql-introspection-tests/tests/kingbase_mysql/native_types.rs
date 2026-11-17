use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseMysql))]
async fn native_type_columns_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE native_types (
            id INT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            small_int SMALLINT NOT NULL,
            tiny_bool TINYINT(1) NOT NULL,
            tiny_int TINYINT NOT NULL,
            medium_int MEDIUMINT NOT NULL,
            big_int BIGINT NOT NULL,
            decimal_value DECIMAL(5, 3) NOT NULL,
            numeric_value NUMERIC(4, 1) NOT NULL,
            float_value FLOAT NOT NULL,
            double_value DOUBLE NOT NULL,
            bits BIT(8) NOT NULL,
            char_value CHAR(10) NOT NULL,
            varchar_value VARCHAR(32) NOT NULL,
            binary_value BINARY(8) NOT NULL,
            varbinary_value VARBINARY(8) NOT NULL,
            tiny_blob TINYBLOB NOT NULL,
            blob_value BLOB NOT NULL,
            medium_blob MEDIUMBLOB NOT NULL,
            long_blob LONGBLOB NOT NULL,
            tiny_text TINYTEXT NOT NULL,
            text_value TEXT NOT NULL,
            medium_text MEDIUMTEXT NOT NULL,
            long_text LONGTEXT NOT NULL,
            date_value DATE NOT NULL,
            time_value TIME(3) NOT NULL,
            datetime_value DATETIME(3) NOT NULL,
            timestamp_value TIMESTAMP(3) NOT NULL,
            year_value YEAR NOT NULL,
            json_value JSON NOT NULL
        );
    "#,
    )
    .await;

    let dml = api.introspect_dml().await?;
    let normalized = dml
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");

    for expected in [
        "small_int Int @db.SmallInt",
        "tiny_bool Int @db.TinyInt",
        "tiny_int Int @db.TinyInt",
        "medium_int Int @db.MediumInt",
        "big_int BigInt",
        "decimal_value Decimal @db.Decimal(5, 3)",
        "numeric_value Decimal @db.Decimal(4, 1)",
        "float_value Float @db.Float",
        "double_value Float",
        "bits Bytes @db.Bit(8)",
        "char_value String @db.Char(10)",
        "varchar_value String @db.VarChar(32)",
        "binary_value Bytes @db.Binary(8)",
        "varbinary_value Bytes @db.VarBinary(8)",
        "tiny_blob Bytes @db.TinyBlob",
        "blob_value Bytes @db.Blob",
        "medium_blob Bytes @db.MediumBlob",
        "long_blob Bytes",
        "tiny_text String @db.TinyText",
        "text_value String @db.Text",
        "medium_text String @db.MediumText",
        "long_text String @db.LongText",
        "date_value DateTime @db.Date",
        "time_value DateTime @db.Time(3)",
        "datetime_value DateTime",
        "timestamp_value DateTime @db.Timestamp(3)",
        "year_value Int @db.Year",
        "json_value Json",
    ] {
        assert!(
            normalized.lines().any(|line| line == expected),
            "missing exact field in:\\n{dml}"
        );
    }

    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn full_column_text_and_blob_keys_introspect_to_valid_psl(api: &mut TestApi) -> TestResult {
    for (index, native_type) in [
        "TEXT",
        "TINYTEXT",
        "MEDIUMTEXT",
        "LONGTEXT",
        "BLOB",
        "TINYBLOB",
        "MEDIUMBLOB",
        "LONGBLOB",
    ]
    .into_iter()
    .enumerate()
    {
        for (shape, columns) in [("single", "value"), ("compound", "value, other")] {
            for kind in ["normal", "unique", "primary"] {
                let table = format!("key_{index}_{shape}_{kind}");
                let key = match kind {
                    "normal" => format!("CREATE INDEX {table}_ix ON {table}({columns});"),
                    "unique" => format!("CREATE UNIQUE INDEX {table}_ix ON {table}({columns});"),
                    _ => String::new(),
                };
                let id = if kind == "primary" {
                    format!(", PRIMARY KEY({columns})")
                } else {
                    ", id INT PRIMARY KEY".into()
                };
                api.raw_cmd(&format!(
                    "CREATE TABLE {table}(value {native_type} NOT NULL, other {native_type} NOT NULL {id}); {key}"
                ))
                .await;
            }
        }
    }
    // Parsing the complete introspected schema catches invalid native-key rules,
    // including default native types whose @db annotation is omitted.
    let schema = api.introspect().await?;
    let parsed = psl::parse_schema_without_extensions(&schema).unwrap();
    assert_eq!(parsed.db.walk_models().count(), 48);
    for model in parsed.db.walk_models() {
        if model.name().ends_with("primary") {
            assert!(model.primary_key().is_some(), "{}", model.name());
        } else {
            let indexes = model.indexes().collect::<Vec<_>>();
            assert_eq!(indexes.len(), 1, "{}", model.name());
            assert_eq!(indexes[0].is_unique(), model.name().ends_with("unique"));
        }
    }
    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn extended_native_parameters_introspect_to_valid_psl(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        "CREATE TABLE native_limits(id INT PRIMARY KEY, c CHAR(256), v VARCHAR(655350), \
         p DECIMAL(66,20), s DECIMAL(44,33), n DECIMAL(2,4), c0 CHAR(0), v0 VARCHAR(0), \
         max_char CHAR(10485760), max_varchar VARCHAR(10485760), max_decimal DECIMAL(1000,1000));",
    )
    .await;
    let schema = api.introspect().await?;
    psl::parse_schema_without_extensions(&schema).unwrap();
    let expected = expect![[r#"
        model native_limits {
          id          Int      @id
          c           String?  @db.Char(256)
          v           String?  @db.VarChar(655350)
          p           Decimal? @db.Decimal(66, 20)
          s           Decimal? @db.Decimal(44, 33)
          n           Decimal? @db.Decimal(4, 4)
          c0          String?  @db.Char(0)
          v0          String?  @db.VarChar(0)
          max_char    String?  @db.Char(10485760)
          max_varchar String?  @db.VarChar(10485760)
          max_decimal Decimal? @db.Decimal(1000, 1000)
        }
    "#]];
    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}
