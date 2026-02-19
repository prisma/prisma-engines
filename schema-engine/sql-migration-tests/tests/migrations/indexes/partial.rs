use sql_migration_tests::test_api::*;

const PREVIEW_FEATURES: &[&str] = &["partialIndexes"];

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn partial_unique_index_on_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("(status = 'active'::text)")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn partial_normal_index_on_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@index([email], where: raw("status IS NOT NULL"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_not_unique().assert_predicate("(status IS NOT NULL)")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn compound_partial_unique_index_on_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id        Int    @id
            firstName String
            lastName  String
            status    String

            @@unique([firstName, lastName], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["firstName", "lastName"], |index| {
            index.assert_is_unique().assert_predicate("(status = 'active'::text)")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn modify_partial_index_predicate_postgres(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'verified'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("(status = 'verified'::text)")
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn remove_partial_index_predicate_postgres(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| index.assert_is_unique().assert_no_predicate())
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn add_partial_index_predicate_postgres(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("(status = 'active'::text)")
        })
    });
}

#[test_connector(tags(Sqlite))]
fn partial_unique_index_on_sqlite(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("status = 'active'")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn partial_normal_index_on_sqlite(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@index([email], where: raw("status IS NOT NULL"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_not_unique().assert_predicate("status IS NOT NULL")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn compound_partial_unique_index_on_sqlite(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id        Int    @id
            firstName String
            lastName  String
            status    String

            @@unique([firstName, lastName], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["firstName", "lastName"], |index| {
            index.assert_is_unique().assert_predicate("status = 'active'")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn modify_partial_index_predicate_sqlite(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'verified'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("status = 'verified'")
        })
    });
}

#[test_connector(tags(Sqlite))]
fn remove_partial_index_predicate_sqlite(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| index.assert_is_unique().assert_no_predicate())
    });
}

#[test_connector(tags(Sqlite))]
fn add_partial_index_predicate_sqlite(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("status = 'active'")
        })
    });
}

#[test_connector(tags(Mssql))]
fn partial_unique_index_on_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("[status]='active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("([status]='active')")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn partial_normal_index_on_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@index([email], where: raw("[status] IS NOT NULL"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_not_unique().assert_predicate("([status] IS NOT NULL)")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn compound_partial_unique_index_on_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id        Int    @id
            firstName String
            lastName  String
            status    String

            @@unique([firstName, lastName], where: raw("[status]='active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["firstName", "lastName"], |index| {
            index.assert_is_unique().assert_predicate("([status]='active')")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn modify_partial_index_predicate_mssql(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("[status]='active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("[status]='verified'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("([status]='verified')")
        })
    });
}

#[test_connector(tags(Mssql))]
fn remove_partial_index_predicate_mssql(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("[status]='active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| index.assert_is_unique().assert_no_predicate())
    });
}

#[test_connector(tags(Mssql))]
fn add_partial_index_predicate_mssql(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email])
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("[status]='active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("([status]='active')")
        })
    });
}

#[test_connector(tags(CockroachDb))]
fn partial_unique_index_on_cockroachdb(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@unique([email], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| index.assert_is_unique().assert_is_partial())
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn partial_normal_index_on_cockroachdb(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@index([email], where: raw("status IS NOT NULL"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| index.assert_is_not_unique().assert_is_partial())
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn compound_partial_unique_index_on_cockroachdb(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id        Int    @id
            firstName String
            lastName  String
            status    String

            @@unique([firstName, lastName], where: raw("status = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["firstName", "lastName"], |index| {
            index.assert_is_unique().assert_is_partial()
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn partial_index_object_literal_camel_case_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id       Int     @id
            email    String
            isActive Boolean

            @@unique([email], where: { isActive: true })
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("(\"isActive\" = true)")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn partial_index_object_literal_camel_case_sqlite(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id       Int     @id
            email    String
            isActive Boolean

            @@unique([email], where: { isActive: true })
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("\"isActive\" = true")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn partial_index_object_literal_camel_case_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id       Int     @id
            email    String
            isActive Boolean

            @@unique([email], where: { isActive: true })
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("User", |table| {
        table.assert_index_on_columns(&["email"], |index| {
            index.assert_is_unique().assert_predicate("([isActive]=(1))")
        })
    });

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn partial_index_with_enum_cast_is_idempotent_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model ProductionItem {
            id     Int              @id
            status ProductionStatus @default(PLANNED)

            @@index([status], where: raw("status != 'COMPLETED'::production_status"))
            @@map("production_item")
        }

        enum ProductionStatus {
            PLANNED
            IN_PRODUCTION
            READY
            COMPLETED

            @@map("production_status")
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn partial_index_with_numeric_literal_cast_is_idempotent_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model OrderLineItemAllocation {
            id                Int     @id
            lineItemId        Int     @map("line_item_id")
            containerQuantity Decimal @map("container_quantity")
            pulledQuantity    Decimal @map("pulled_quantity")

            @@index([lineItemId, containerQuantity, pulledQuantity], where: raw("container_quantity > 0"))
            @@map("order_line_item_allocation")
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}
