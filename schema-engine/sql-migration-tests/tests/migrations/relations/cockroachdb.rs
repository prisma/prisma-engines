use sql_migration_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
fn adding_mutual_references_on_existing_tables_works(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
        }

        model B {
            id Int @id
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model A {
            id Int @id
            name String @unique
            b_email String
            brel B @relation("AtoB", fields: [b_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
            b    B[] @relation("BtoA")
        }

        model B {
            id Int @id
            email String @unique
            a_name String
            arel A @relation("BtoA", fields: [a_name], references: [name], onDelete: NoAction, onUpdate: NoAction)
            a    A[] @relation("AtoB")
        }
    "#;

    let res = api.schema_push_w_datasource(dm2).force(true).send();

    res.assert_warnings(&["A unique constraint covering the columns `[name]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into(), "A unique constraint covering the columns `[email]` on the table `B` will be added. If there are existing duplicate values, this will fail.".into()]);
}
