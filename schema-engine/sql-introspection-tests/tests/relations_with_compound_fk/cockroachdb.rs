use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(CockroachDb243))]
async fn compound_foreign_keys_with_defaults_v_24_3(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE "Person" (
            id          INT8 GENERATED ALWAYS AS IDENTITY,
            age         INTEGER NOT NULL,
            partner_id  INT4 NOT NULL DEFAULT 0,
            partner_age INT4 NOT NULL DEFAULT 0,

            CONSTRAINT post_user_unique UNIQUE (id, age),
            CONSTRAINT "Person_partner_id_partner_age_fkey" FOREIGN KEY (partner_id, partner_age) REFERENCES "Person"(id, age),
            CONSTRAINT "Person_pkey" PRIMARY KEY (id)
        );
    "#,
    )
    .await;

    let expected = expect![[r#"
        model Person {
          id           BigInt   @id @default(sequence(maxValue: 2147483647))
          age          Int
          partner_id   Int      @default(0)
          partner_age  Int      @default(0)
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb251))]
async fn compound_foreign_keys_with_defaults_v_25_1(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE "Person" (
            id          INT8 GENERATED ALWAYS AS IDENTITY,
            age         INTEGER NOT NULL,
            partner_id  INT4 NOT NULL DEFAULT 0,
            partner_age INT4 NOT NULL DEFAULT 0,

            CONSTRAINT post_user_unique UNIQUE (id, age),
            CONSTRAINT "Person_partner_id_partner_age_fkey" FOREIGN KEY (partner_id, partner_age) REFERENCES "Person"(id, age),
            CONSTRAINT "Person_pkey" PRIMARY KEY (id)
        );
    "#,
    )
    .await;

    let expected = expect![[r#"
        model Person {
          id           BigInt   @id @default(sequence(maxValue: 2147483647))
          age          Int
          partner_id   Int      @default(0)
          partner_age  Int      @default(0)
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
