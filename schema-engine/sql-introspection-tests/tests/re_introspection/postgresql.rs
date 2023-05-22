use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn re_introspecting_custom_compound_id_names(api: &mut TestApi) -> TestResult {
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
        *** WARNING ***

        These models were enriched with custom compound id names taken from the previous Prisma schema:
          - "User"
          - "User2"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn re_introspecting_custom_compound_unique_names(api: &mut TestApi) -> TestResult {
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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn mapped_enum_value_name(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE Type color as ENUM ('black', 'white');

        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            color color NOT NULL DEFAULT 'black'
        );

        CREATE TABLE "Unrelated" (
            id SERIAL PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model User {
          id    Int   @id @default(autoincrement())
          color color @default(BLACK)
        }

        enum color {
          BLACK @map("black")
          white
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id    Int   @id @default(autoincrement())
          color color @default(BLACK)
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum color {
          BLACK @map("black")
          white
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These enum values were enriched with `@map` information taken from the previous Prisma schema:
          - Enum: "color", value: "BLACK"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn ignore_docs_only_added_once(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "A" (
            id INT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model A {
          id Int?

          @@ignore
        }
    "#};

    let expectation = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model A {
          id Int?

          @@ignore
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![""];
    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reserved_name_docs_are_only_added_once(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "if" (
            id INT PRIMARY KEY
        );
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        /// This model has been renamed to Renamedif during introspection, because the original name if is reserved.
        model Renamedif {
          id Int @id

          @@map("if")
        }
    "#};

    let expectation = expect![[r#"
        /// This model has been renamed to Renamedif during introspection, because the original name if is reserved.
        model Renamedif {
          id Int @id

          @@map("if")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "Renamedif"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn re_introspecting_uuid_default_on_uuid_typed_pk_field(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "mymodel" (
            id UUID PRIMARY KEY
        );
    "#};

    let prisma_schema = r#"
        model mymodel {
            id String @id @default(uuid()) @db.Uuid
        }
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model mymodel {
          id String @id @default(uuid()) @db.Uuid
        }
    "#]];

    api.expect_re_introspected_datamodel(prisma_schema, expected).await;
    Ok(())
}
