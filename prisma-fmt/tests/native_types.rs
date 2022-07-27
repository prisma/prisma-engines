use expect_test::*;

#[test]
fn test_native_types_list_on_crdb() {
    let schema = r#"
        datasource mydb {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }
    "#;

    let result = prisma_fmt::native_types(schema.to_owned());
    let expected = expect![[
        r#"[{"name":"Bit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Char","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Decimal","_number_of_args":0,"_number_of_optional_args":2,"prisma_types":["Decimal"]},{"name":"String","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Timestamp","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timestamptz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Time","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"Timetz","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["DateTime"]},{"name":"VarBit","_number_of_args":0,"_number_of_optional_args":1,"prisma_types":["String"]},{"name":"Bool","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Boolean"]},{"name":"Bytes","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Bytes"]},{"name":"Date","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["DateTime"]},{"name":"Float4","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"Float8","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Float"]},{"name":"Inet","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Int2","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Int4","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"Int8","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["BigInt"]},{"name":"JsonB","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Json"]},{"name":"Oid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["Int"]},{"name":"CatalogSingleChar","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]},{"name":"Uuid","_number_of_args":0,"_number_of_optional_args":0,"prisma_types":["String"]}]"#
    ]];
    expected.assert_eq(&result);
}
