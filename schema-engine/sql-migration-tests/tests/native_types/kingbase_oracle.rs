use psl::builtin_connectors::KINGBASE_ORACLE;
use sql_migration_tests::test_api::*;
use sql_schema_describer::ColumnTypeFamily;

#[test_connector(tags(KingbaseOracle))]
fn all_supported_oracle_native_types_can_be_created_and_reintrospected(api: TestApi) {
    let schema = r#"
        model NativeTypes {
            id                    Int       @id
            tinyIntValue          Int?      @db.TinyInt
            intValue              Int?      @db.Number(10, 0)
            bigintValue           BigInt?   @db.Number(19, 0)
            decimalValue          Decimal?  @db.Number(12, 4)
            floatValue            Float?    @db.Float
            binaryFloatValue      Float?    @db.BinaryFloat
            binaryDoubleValue     Float?    @db.BinaryDouble
            charValue             String?   @db.Char(10)
            varcharValue          String?   @db.VarChar2(32)
            ncharValue            String?   @db.NChar(10)
            nvarcharValue         String?   @db.NVarChar2(32)
            clobValue             String?   @db.Clob
            nclobValue            String?   @db.NClob
            blobValue             Bytes?    @db.Blob
            dateValue             DateTime? @db.Date
            timestampValue        DateTime? @db.Timestamp(3)
            timestampTzValue      DateTime? @db.TimestampTz(3)
            timestampLocalTzValue DateTime? @db.TimestampLocalTz(3)
            boolValue             Boolean?  @db.Boolean
            jsonValue             Json?     @db.Json
            uuidValue             String?   @db.Uuid
            xmlValue              String?   @db.Xml
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("NativeTypes", |table| {
        table
            .assert_columns_count(23)
            .assert_column("id", |column| column.assert_type_family(ColumnTypeFamily::Int))
            .assert_column("tinyIntValue", |column| {
                column.assert_native_type("TinyInt", KINGBASE_ORACLE)
            })
            .assert_column("intValue", |column| column.assert_type_family(ColumnTypeFamily::Int))
            .assert_column("bigintValue", |column| {
                column.assert_type_family(ColumnTypeFamily::BigInt)
            })
            .assert_column("decimalValue", |column| {
                column.assert_type_family(ColumnTypeFamily::Decimal)
            })
            .assert_column("floatValue", |column| {
                column.assert_type_family(ColumnTypeFamily::Float)
            })
            .assert_column("binaryFloatValue", |column| {
                column.assert_native_type("BinaryFloat", KINGBASE_ORACLE)
            })
            .assert_column("binaryDoubleValue", |column| {
                column.assert_native_type("BinaryDouble", KINGBASE_ORACLE)
            })
            .assert_column("charValue", |column| {
                column.assert_native_type("Char(10)", KINGBASE_ORACLE)
            })
            .assert_column("varcharValue", |column| {
                column.assert_native_type("VarChar2(32)", KINGBASE_ORACLE)
            })
            // C010 canonicalizes national CHAR/VARCHAR aliases; NCLOB retains its physical identity.
            .assert_column("ncharValue", |column| {
                column.assert_native_type("Char(10)", KINGBASE_ORACLE)
            })
            .assert_column("nvarcharValue", |column| {
                column.assert_native_type("VarChar2(32)", KINGBASE_ORACLE)
            })
            .assert_column("clobValue", |column| column.assert_native_type("Clob", KINGBASE_ORACLE))
            .assert_column("nclobValue", |column| {
                column.assert_native_type("NClob", KINGBASE_ORACLE)
            })
            .assert_column("blobValue", |column| column.assert_native_type("Blob", KINGBASE_ORACLE))
            .assert_column("dateValue", |column| column.assert_native_type("Date", KINGBASE_ORACLE))
            .assert_column("timestampTzValue", |column| {
                column.assert_native_type("TimestampTz(3)", KINGBASE_ORACLE)
            })
            // TIMESTAMP WITH LOCAL TIME ZONE is reported as a timestamp by this catalog.
            .assert_column("timestampLocalTzValue", |column| {
                column.assert_native_type("Timestamp(3)", KINGBASE_ORACLE)
            })
            .assert_column("boolValue", |column| {
                column.assert_native_type("Boolean", KINGBASE_ORACLE)
            })
            .assert_column("jsonValue", |column| column.assert_native_type("Json", KINGBASE_ORACLE))
            .assert_column("uuidValue", |column| column.assert_native_type("Uuid", KINGBASE_ORACLE))
            .assert_column("xmlValue", |column| column.assert_native_type("Xml", KINGBASE_ORACLE))
    });

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}
