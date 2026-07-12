use quaint::{ValueType, prelude::Insert};
use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultValue;

#[test_connector(tags(Sqlite))]
fn changing_a_column_from_optional_to_required_with_a_default_is_safe(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Test"), ["id", "age"])
        .values(("a", 12))
        .values(("b", 22))
        .values(("c", ValueType::Int32(None)));

    api.query(insert.into());

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send().assert_green();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("age", |column| {
            column
                .assert_default(Some(DefaultValue::value(30)))
                .assert_is_required()
        })
    });

    // Check that no data was lost.
    {
        let data = api.dump_table("Test");
        assert_eq!(data.len(), 3);
        let ages: Vec<Option<i64>> = data
            .into_iter()
            .map(|row| row.get("age").unwrap().as_integer())
            .collect();

        // TODO: this is NOT what users would expect (it's a consequence of the stepped migration
        // process), we should have a more specific warning for this.
        assert_eq!(ages, &[Some(12), Some(22), Some(30)]);
    }
}
