use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn re_introspecting_custom_compound_id_names(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT "User.something@invalid-and/weird" PRIMARY KEY (first, last)
        );

        CREATE TABLE "User2" (
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT "User2_pkey" PRIMARY KEY (first, last)
        );

        CREATE TABLE "Unrelated" (
            id SERIAL PRIMARY KEY
        )
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
         model User {
           first  Int
           last   Int

           @@id([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model User2 {
           first  Int
           last   Int

           @@id([first, last], name: "compound")
         }
     "#};

    let expectation = expect![[r#"
         model User {
           first Int
           last  Int

           @@id([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model User2 {
           first Int
           last  Int

           @@id([first, last], name: "compound")
         }

         model Unrelated {
           id Int @id @default(autoincrement())
         }
     "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expected = expect![[r#"
        [
          {
            "code": 18,
            "message": "These models were enriched with custom compound id names taken from the previous Prisma schema.",
            "affected": [
              {
                "model": "User"
              },
              {
                "model": "User2"
              }
            ]
          }
        ]"#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn re_introspecting_custom_compound_unique_names(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first INT NOT NULL,
            last INT NOT NULL,
            CONSTRAINT "User.something@invalid-and/weird" UNIQUE (first, last)
        );

        CREATE TABLE "Unrelated" (
            id SERIAL PRIMARY KEY
        )
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
         model User {
           id    Int @id @default(autoincrement())
           first Int
           last  Int

           @@unique([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }
     "#};

    let expectation = expect![[r#"
         model User {
           id    Int @id @default(autoincrement())
           first Int
           last  Int

           @@unique([first, last], name: "compound", map: "User.something@invalid-and/weird")
         }

         model Unrelated {
           id Int @id @default(autoincrement())
         }
     "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    Ok(())
}
