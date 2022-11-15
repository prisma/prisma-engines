use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(CockroachDb))]
async fn compound_foreign_keys_with_defaults(api: &TestApi) -> TestResult {
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
          id           BigInt   @id @default(sequence())
          age          Int
          partner_id   Int      @default(0)
          partner_age  Int      @default(0)
          Person       Person   @relation(fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[]

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
