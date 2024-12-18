use std::sync::Arc;

#[test]
fn empty_datamodel_loads() {
    let schema = "";
    let parsed_schema = psl::parse_schema(schema).unwrap();
    let schema = schema::build(Arc::new(parsed_schema), true);
    assert!(!schema.is_mongo());
    assert!(!schema.is_sql());
}
