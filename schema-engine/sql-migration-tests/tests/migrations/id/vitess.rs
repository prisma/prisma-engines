use sql_migration_tests::test_api::*;

#[test_connector(tags(Vitess))]
fn changing_the_type_of_an_id_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_column("b_id", |col| col.assert_type_is_int()));

    let dm2 = r#"
        model A {
            id Int @id
            b_id String
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id String @id @default(cuid())
            a  A[]

        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("b_id", |col| col.assert_type_is_string())
    });
}
