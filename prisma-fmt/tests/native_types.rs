use expect_test::*;

#[test]
fn test_native_types_list_on_crdb() {
    let schema = r#"
        datasource mydb {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }
    "#;

    let result = prisma_fmt::native_types(serde_json::to_string(schema).unwrap());
    let expected = expect![[
        r#"[{"name":"Bit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Bool","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Boolean"]},{"name":"Bytes","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Bytes"]},{"name":"Char","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Date","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["DateTime"]},{"name":"Decimal","_number_of_args":0,"_number_of_optional_args":2,"prisma_types":["Decimal"]},{"name":"Float4","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"Float8","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"Inet","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Int2","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Int4","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Int8","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["BigInt"]},{"name":"JsonB","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Json"]},{"name":"Oid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"CatalogSingleChar","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"String","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Time","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timestamp","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timestamptz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timetz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Uuid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"VarBit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]}]"#
    ]];
    expected.assert_eq(&result);
}

#[test]
fn test_native_types_multifile() {
    let schema = &[
        (
            "A.prisma",
            r#"
        datasource mydb {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }"#,
        ),
        (
            "B.prisma",
            r#"
        model M {
          id String @id
        }"#,
        ),
    ];

    let result = prisma_fmt::native_types(serde_json::to_string(schema).unwrap());
    let expected = expect![[
        r#"[{"name":"SmallInt","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Integer","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"BigInt","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["BigInt"]},{"name":"Decimal","_number_of_args":0,"_number_of_optional_args":2,"prisma_types":["Decimal"]},{"name":"Money","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Decimal"]},{"name":"Inet","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Oid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Citext","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Real","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"DoublePrecision","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"VarChar","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Char","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Text","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"ByteA","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Bytes"]},{"name":"Timestamp","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timestamptz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Date","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["DateTime"]},{"name":"Time","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timetz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Boolean","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Boolean"]},{"name":"Bit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"VarBit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Uuid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Xml","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Json","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Json"]},{"name":"JsonB","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Json"]}]"#
    ]];
    expected.assert_eq(&result);
}
