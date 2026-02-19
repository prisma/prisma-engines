use sql_migration_tests::test_api::*;

const PREVIEW_FEATURES: &[&str] = &["partialIndexes"];

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Sqlite), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
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

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
fn partial_index_with_whitespace_normalization_is_idempotent_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model User {
            id     Int    @id
            email  String
            status String

            @@index([email], where: raw("[status] = 'active'"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql), preview_features("partialIndexes"))]
fn partial_index_with_comparison_normalization_is_idempotent_mssql(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model OrderLineItemAllocation {
            id                Int     @id
            lineItemId        Int     @map("line_item_id")
            containerQuantity Decimal @map("container_quantity")
            pulledQuantity    Decimal @map("pulled_quantity")

            @@index([lineItemId, containerQuantity, pulledQuantity], where: raw("[container_quantity] > 0"))
            @@map("order_line_item_allocation")
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn db_partial_index_not_recreated_without_preview_feature_postgres(api: TestApi) {
    let schema_name = api.schema_name();
    let dm = api.datamodel_with_provider(
        r#"model User {
            id    Int    @id
            email String

            @@index([email])
        }"#,
    );

    api.schema_push(&dm).send().assert_green();

    api.raw_cmd(&format!(
        "DROP INDEX \"{schema_name}\".\"User_email_idx\"; CREATE INDEX \"User_email_idx\" ON \"{schema_name}\".\"User\" (email) WHERE email IS NOT NULL;"
    ));

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn db_partial_index_not_recreated_without_preview_feature_mssql(api: TestApi) {
    let schema = api.schema_name();
    let dm = api.datamodel_with_provider(
        r#"model User {
            id    Int    @id
            email String

            @@index([email])
        }"#,
    );

    api.schema_push(&dm).send().assert_green();

    api.raw_cmd(&format!(
        "DROP INDEX [User_email_idx] ON [{schema}].[User]; CREATE INDEX [User_email_idx] ON [{schema}].[User] ([email]) WHERE [email] IS NOT NULL;"
    ));

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn db_partial_index_not_recreated_without_preview_feature_sqlite(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"model User {
            id    Int    @id
            email String

            @@index([email])
        }"#,
    );

    api.schema_push(&dm).send().assert_green();

    api.raw_cmd(
        "DROP INDEX \"User_email_idx\"; CREATE INDEX \"User_email_idx\" ON \"User\" (email) WHERE email IS NOT NULL;",
    );

    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
fn partial_index_with_lossy_float_to_int_cast_is_idempotent_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model Item {
            id    Int     @id
            price Decimal

            @@index([price], where: raw("price > 3.14::integer"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
fn partial_index_with_double_cast_truncation_is_idempotent_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model Item {
            id    Int     @id
            price Decimal

            @@index([price], where: raw("price > 3.14::integer::numeric"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
fn partial_index_with_back_and_forth_lossy_cast_is_idempotent_postgres(api: TestApi) {
    let dm = api.datamodel_with_provider_and_features(
        r#"model Item {
            id    Int     @id
            price Decimal

            @@index([price], where: raw("price > 99.9::smallint::numeric"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(&dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
fn partial_index_cast_type_change_triggers_migration_postgres(api: TestApi) {
    let dm1 = api.datamodel_with_provider_and_features(
        r#"model Item {
            id    Int     @id
            price Decimal

            @@index([price], where: raw("price > 3.14::integer"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm1).send().assert_green();
    api.schema_push(&dm1).send().assert_no_steps();

    let dm2 = api.datamodel_with_provider_and_features(
        r#"model Item {
            id    Int     @id
            price Decimal

            @@index([price], where: raw("price > 3.14::numeric"))
        }"#,
        &[],
        PREVIEW_FEATURES,
    );

    api.schema_push(&dm2).force(true).send().assert_has_executed_steps();
    api.schema_push(&dm2).send().assert_no_steps();
}
