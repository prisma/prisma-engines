use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_foreign_keys_and_delete_actions_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY,
            code VARCHAR2(16) NOT NULL,
            CONSTRAINT parent_code_key UNIQUE (code)
        );

        CREATE TABLE child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL,
            parent_code VARCHAR2(16),
            CONSTRAINT child_parent_id_fkey
                FOREIGN KEY (parent_id) REFERENCES parent (id) ON DELETE CASCADE,
            CONSTRAINT child_parent_code_fkey
                FOREIGN KEY (parent_code) REFERENCES parent (code) ON DELETE SET NULL
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        model child {
          id                               Int     @id
          parent_id                        Int
          parent_code                      String? @db.VarChar2(16)
          parent_child_parent_codeToparent parent? @relation("child_parent_codeToparent", fields: [parent_code], references: [code], onUpdate: NoAction)
          parent_child_parent_idToparent   parent  @relation("child_parent_idToparent", fields: [parent_id], references: [id], onDelete: Cascade, onUpdate: NoAction)
        }

        model parent {
          id                              Int     @id
          code                            String  @unique @db.VarChar2(16)
          child_child_parent_codeToparent child[] @relation("child_parent_codeToparent")
          child_child_parent_idToparent   child[] @relation("child_parent_idToparent")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_one_to_one_self_and_compound_relations_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE account (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE profile (
            id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL UNIQUE,
            CONSTRAINT profile_account_fk FOREIGN KEY (account_id) REFERENCES account (id)
        );

        CREATE TABLE employee (
            id INTEGER PRIMARY KEY,
            manager_id INTEGER,
            CONSTRAINT employee_manager_fk FOREIGN KEY (manager_id) REFERENCES employee (id)
        );

        CREATE TABLE tenant_user (
            tenant_id INTEGER NOT NULL,
            id INTEGER NOT NULL,
            CONSTRAINT tenant_user_pk PRIMARY KEY (tenant_id, id)
        );

        CREATE TABLE membership (
            id INTEGER PRIMARY KEY,
            tenant_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            CONSTRAINT membership_tenant_user_fk
                FOREIGN KEY (tenant_id, user_id) REFERENCES tenant_user (tenant_id, id)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "account_id Int     @unique",
        "fields: [account_id], references: [id]",
        "manager_id",
        "fields: [manager_id], references: [id]",
        "@@id([tenant_id, id]",
        "fields: [tenant_id, user_id], references: [tenant_id, id]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_explicit_join_models_and_multiple_foreign_keys_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE app_user (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE post (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE post_assignment (
            id INTEGER PRIMARY KEY,
            author_id INTEGER NOT NULL,
            reviewer_id INTEGER NOT NULL,
            post_id INTEGER NOT NULL,
            CONSTRAINT post_assignment_author_fk FOREIGN KEY (author_id) REFERENCES app_user (id),
            CONSTRAINT post_assignment_reviewer_fk FOREIGN KEY (reviewer_id) REFERENCES app_user (id),
            CONSTRAINT post_assignment_post_fk FOREIGN KEY (post_id) REFERENCES post (id)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "model post_assignment",
        "fields: [author_id], references: [id]",
        "fields: [reviewer_id], references: [id]",
        "fields: [post_id], references: [id]",
        "@relation(\"post_assignment_author",
        "@relation(\"post_assignment_reviewer",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_prisma_implicit_many_to_many_tables_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "Post" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "_PostToUser" (
            "A" INTEGER NOT NULL,
            "B" INTEGER NOT NULL,
            CONSTRAINT "_PostToUser_A_fkey" FOREIGN KEY ("A") REFERENCES "Post" (id),
            CONSTRAINT "_PostToUser_B_fkey" FOREIGN KEY ("B") REFERENCES "User" (id),
            CONSTRAINT "_PostToUser_AB_pkey" PRIMARY KEY ("A", "B")
        );

        CREATE INDEX "_PostToUser_B_index" ON "_PostToUser" ("B");
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in ["model Post", "model User", "User User[]", "Post Post[]"] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }
    assert!(!result.contains("model _PostToUser"), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_multiple_compound_foreign_keys_have_stable_relation_names(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE customer (
            tenant_id INTEGER NOT NULL,
            id INTEGER NOT NULL,
            CONSTRAINT customer_pk PRIMARY KEY (tenant_id, id)
        );

        CREATE TABLE invoice (
            id INTEGER PRIMARY KEY,
            billing_tenant_id INTEGER NOT NULL,
            billing_customer_id INTEGER NOT NULL,
            shipping_tenant_id INTEGER NOT NULL,
            shipping_customer_id INTEGER NOT NULL,
            CONSTRAINT invoice_billing_customer_fk
                FOREIGN KEY (billing_tenant_id, billing_customer_id)
                REFERENCES customer (tenant_id, id),
            CONSTRAINT invoice_shipping_customer_fk
                FOREIGN KEY (shipping_tenant_id, shipping_customer_id)
                REFERENCES customer (tenant_id, id)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "fields: [billing_tenant_id, billing_customer_id], references: [tenant_id, id]",
        "fields: [shipping_tenant_id, shipping_customer_id], references: [tenant_id, id]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }
    assert_eq!(2, result.matches("references: [tenant_id, id]").count(), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_one_to_one_relations_can_reference_a_non_primary_unique_column(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE account (
            id INTEGER PRIMARY KEY,
            code VARCHAR2(32) NOT NULL,
            CONSTRAINT account_code_key UNIQUE (code)
        );

        CREATE TABLE account_settings (
            id INTEGER PRIMARY KEY,
            account_code VARCHAR2(32) NOT NULL,
            CONSTRAINT account_settings_account_code_key UNIQUE (account_code),
            CONSTRAINT account_settings_account_code_fk
                FOREIGN KEY (account_code) REFERENCES account (code)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for field in ["code", "account_code"] {
        assert!(
            result
                .lines()
                .any(|line| line.contains(field) && line.contains("@unique") && line.contains("@db.VarChar2(32)")),
            "missing unique `{field}` in:\n{result}"
        );
    }
    assert!(
        result.contains("fields: [account_code], references: [code]"),
        "{result}"
    );

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_two_one_to_one_relations_between_the_same_models_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE left_model (
            id INTEGER PRIMARY KEY,
            right_id INTEGER NOT NULL UNIQUE
        );

        CREATE TABLE right_model (
            id INTEGER PRIMARY KEY,
            left_id INTEGER NOT NULL UNIQUE,
            CONSTRAINT right_model_left_fk FOREIGN KEY (left_id) REFERENCES left_model (id)
        );

        ALTER TABLE left_model
            ADD CONSTRAINT left_model_right_fk FOREIGN KEY (right_id) REFERENCES right_model (id);
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "fields: [right_id], references: [id]",
        "fields: [left_id], references: [id]",
        "right_id",
        "left_id",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }
    assert!(result.matches("@relation(\"").count() >= 4, "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_primary_key_foreign_keys_and_compound_one_to_one_relations_are_introspected(
    api: &mut TestApi,
) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY,
            tenant_id INTEGER NOT NULL,
            CONSTRAINT parent_tenant_id_key UNIQUE (tenant_id, id)
        );

        CREATE TABLE child (
            parent_id INTEGER PRIMARY KEY,
            CONSTRAINT child_parent_fk FOREIGN KEY (parent_id) REFERENCES parent (id)
        );

        CREATE TABLE tenant_profile (
            id INTEGER PRIMARY KEY,
            tenant_id INTEGER,
            parent_id INTEGER,
            CONSTRAINT tenant_profile_parent_key UNIQUE (tenant_id, parent_id),
            CONSTRAINT tenant_profile_parent_fk
                FOREIGN KEY (tenant_id, parent_id) REFERENCES parent (tenant_id, id)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    assert!(
        result
            .lines()
            .any(|line| line.trim_start().starts_with("parent_id") && line.contains("@id")),
        "the primary-key foreign key was not rendered as an id:\n{result}"
    );

    for expected in [
        "fields: [parent_id], references: [id]",
        "fields: [tenant_id, parent_id], references: [tenant_id, id]",
        "@@unique([tenant_id, parent_id]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_direct_and_implicit_many_to_many_relations_do_not_clash(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "Event" (
            id INTEGER PRIMARY KEY,
            host_id INTEGER NOT NULL,
            CONSTRAINT "Event_host_id_fkey" FOREIGN KEY (host_id) REFERENCES "User" (id)
        );

        CREATE TABLE "_EventToUser" (
            "A" INTEGER NOT NULL,
            "B" INTEGER NOT NULL,
            CONSTRAINT "_EventToUser_A_fkey" FOREIGN KEY ("A") REFERENCES "Event" (id),
            CONSTRAINT "_EventToUser_B_fkey" FOREIGN KEY ("B") REFERENCES "User" (id),
            CONSTRAINT "_EventToUser_AB_pkey" PRIMARY KEY ("A", "B")
        );

        CREATE INDEX "_EventToUser_B_index" ON "_EventToUser" ("B");
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "model Event",
        "host_id",
        "@relation(\"Event_host_idToUser\"",
        "@relation(\"EventToUser\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }
    assert!(!result.contains("model _EventToUser"), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_compound_self_relations_preserve_fields_references_and_defaults(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE person (
            id INTEGER PRIMARY KEY,
            age INTEGER NOT NULL,
            partner_id INTEGER NOT NULL DEFAULT 0,
            partner_age INTEGER NOT NULL DEFAULT 0,
            CONSTRAINT person_id_age_key UNIQUE (id, age),
            CONSTRAINT person_partner_fk
                FOREIGN KEY (partner_id, partner_age) REFERENCES person (id, age)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "fields: [partner_id, partner_age], references: [id, age]",
        "@@unique([id, age]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }
    for field in ["partner_id", "partner_age"] {
        assert!(
            result
                .lines()
                .any(|line| { line.trim_start().starts_with(field) && line.contains("@default(0)") }),
            "missing default for `{field}` in:\n{result}"
        );
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_non_unique_compound_foreign_keys_are_rendered_as_many_to_one(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE tenant_user (
            tenant_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            CONSTRAINT tenant_user_pk PRIMARY KEY (tenant_id, user_id)
        );

        CREATE TABLE membership (
            id INTEGER PRIMARY KEY,
            tenant_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            CONSTRAINT membership_tenant_user_fk
                FOREIGN KEY (tenant_id, user_id) REFERENCES tenant_user (tenant_id, user_id)
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "model membership",
        "tenant_user tenant_user @relation(fields: [tenant_id, user_id], references: [tenant_id, user_id]",
        "membership membership[]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
