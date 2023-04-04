use sql_migration_tests::test_api::*;

#[test_connector(tags(Vitess))]
fn reordering_and_altering_models_at_the_same_time_works(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc", fields: [name], references: [name], onDelete: Restrict, onUpdate: Restrict)
            cs C[] @relation(name: "ctoa")
        }

        model B {
            id Int @id
            name Int @unique
            c C @relation(name: "btoc", fields: [name], references: [name], onDelete: Restrict)
        }

        model C {
            id Int @id
            name Int @unique
            a A @relation(name: "ctoa", fields: [name], references: [name], onDelete: Restrict)
            as A[] @relation(name: "atoc")
            bs B[] @relation(name: "btoc")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model C {
            id Int @id
            a A @relation(name: "ctoa2", fields: [name], references: [name], onDelete: Restrict)
            name Int @unique
            bs B[] @relation(name: "btoc2")
            as A[] @relation(name: "atoc2")
        }

        model A {
            id Int @id
            name Int @unique
            c C @relation(name: "atoc2", fields: [name], references: [name], onDelete: Restrict, onUpdate: Restrict)
            cs C[] @relation(name: "ctoa2")
        }

        model B {
            c C @relation(name: "btoc2", fields: [name], references: [name], onDelete: Restrict)
            name Int @unique
            id Int @id
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
}
