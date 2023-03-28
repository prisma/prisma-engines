use crate::dmmf_from_schema;

#[test]
fn sqlite_ignore() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/sqlite_ignore.prisma"));

    // The Post model is ignored.
    assert_eq!(dmmf.data_model.models.len(), 1);
    assert_eq!(dmmf.mappings.model_operations.len(), 1);
}

#[test]
fn standupbot_schema_snapshot() {
    let dmmf = crate::dmmf_from_schema(include_str!("../../schema-builder/benches/standupbot.prisma"));
    let expect = expect_test::expect_file!["./test-schemas/standupbot.expected.json"];
    expect.assert_eq(&serde_json::to_string_pretty(&dmmf).unwrap());
}
