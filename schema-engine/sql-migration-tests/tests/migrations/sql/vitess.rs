use sql_migration_tests::test_api::*;

#[test_connector(tags(Vitess))]
fn dropping_mutually_referencing_tables_works(api: TestApi) {
    let dm1 = r#"
    model A {
        id Int @id
        b_id Int
        ab B @relation("AtoB", fields: [b_id], references: [id])
        c_id Int
        ac C @relation("AtoC", fields: [c_id], references: [id])
        b  B[] @relation("BtoA")
        c  C[] @relation("CtoA")
    }

    model B {
        id Int @id
        a_id Int
        ba A @relation("BtoA", fields: [a_id], references: [id], onUpdate: NoAction)
        c_id Int
        bc C @relation("BtoC", fields: [c_id], references: [id])
        a  A[] @relation("AtoB")
        c  C[] @relation("CtoB")
    }

    model C {
        id Int @id
        a_id Int
        ca A @relation("CtoA", fields: [a_id], references: [id], onUpdate: NoAction)
        b_id Int
        cb B @relation("CtoB", fields: [b_id], references: [id], onUpdate: NoAction)
        b  B[] @relation("BtoC")
        a  A[] @relation("AtoC")
    }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_tables_count(3);

    api.schema_push_w_datasource("").send().assert_green();
    api.assert_schema().assert_tables_count(0);
}
