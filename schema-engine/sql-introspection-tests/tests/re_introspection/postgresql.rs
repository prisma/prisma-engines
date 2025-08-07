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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_partial_indexes_basic(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255),
            active BOOLEAN NOT NULL DEFAULT false
        );

        CREATE INDEX "User_email_idx" ON "User" (email) WHERE email IS NOT NULL;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model User {
          id     Int      @id @default(autoincrement())
          email  String?  @db.VarChar(255)
          active Boolean  @default(false)

          @@index([email], where: "email IS NOT NULL")
        }
    "#};

    let expectation = expect![[r#"
        model User {
          id     Int     @id @default(autoincrement())
          email  String? @db.VarChar(255)
          active Boolean @default(false)

          @@index([email], where: "(email IS NOT NULL)")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_partial_unique_constraints(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Post" (
            id SERIAL PRIMARY KEY,
            slug VARCHAR(255),
            published BOOLEAN NOT NULL DEFAULT false
        );

        CREATE UNIQUE INDEX "Post_slug_key" ON "Post" (slug) WHERE published = true;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model Post {
          id        Int      @id @default(autoincrement())
          slug      String?  @db.VarChar(255)
          published Boolean  @default(false)

          @@unique([slug], where: "published = true")
        }
    "#};

    let expectation = expect![[r#"
        model Post {
          id        Int     @id @default(autoincrement())
          slug      String? @unique @db.VarChar(255)
          published Boolean @default(false)
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_compound_partial_indexes(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Order" (
            id SERIAL PRIMARY KEY,
            user_id INT NOT NULL,
            status VARCHAR(20) NOT NULL,
            created_at TIMESTAMP
        );

        CREATE INDEX "Order_user_status_idx" ON "Order" (user_id, status) WHERE status IN ('pending', 'processing');
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model Order {
          id         Int       @id @default(autoincrement())
          user_id    Int
          status     String    @db.VarChar(20)
          created_at DateTime? @db.Timestamp(6)

          @@index([user_id, status], where: "status IN ('pending', 'processing')")
        }
    "#};

    let expectation = expect![[r#"
        model Order {
          id         Int       @id @default(autoincrement())
          user_id    Int
          status     String    @db.VarChar(20)
          created_at DateTime? @db.Timestamp(6)

          @@index([user_id, status], map: "Order_user_status_idx", where: "((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying])::text[]))")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_partial_indexes_with_custom_names(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Product" (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255),
            price DECIMAL(10,2),
            active BOOLEAN DEFAULT true
        );

        CREATE INDEX "Product.custom@name" ON "Product" (name) WHERE active = true;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model Product {
          id     Int      @id @default(autoincrement())
          name   String?  @db.VarChar(255)
          price  Decimal? @db.Decimal(10, 2)
          active Boolean? @default(true)

          @@index([name], map: "Product.custom@name", where: "active = true")
        }
    "#};

    let expectation = expect![[r#"
        model Product {
          id     Int      @id @default(autoincrement())
          name   String?  @db.VarChar(255)
          price  Decimal? @db.Decimal(10, 2)
          active Boolean? @default(true)

          @@index([name], map: "Product.custom@name", where: "(active = true)")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_partial_indexes_with_gin_algorithm(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Document" (
            id SERIAL PRIMARY KEY,
            tags TEXT[],
            published BOOLEAN DEFAULT false
        );

        CREATE INDEX "Document_tags_idx" ON "Document" USING GIN (tags) WHERE published = true;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model Document {
          id        Int       @id @default(autoincrement())
          tags      String[]
          published Boolean?  @default(false)

          @@index([tags], type: Gin, where: "published = true")
        }
    "#};

    let expectation = expect![[r#"
        model Document {
          id        Int      @id @default(autoincrement())
          tags      String[]
          published Boolean? @default(false)

          @@index([tags], type: Gin, where: "(published = true)")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspecting_multiple_partial_indexes_same_table(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Task" (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255),
            priority INT,
            completed BOOLEAN DEFAULT false,
            archived BOOLEAN DEFAULT false
        );

        CREATE INDEX "Task_title_idx" ON "Task" (title) WHERE completed = false;
        CREATE INDEX "Task_priority_idx" ON "Task" (priority) WHERE priority > 3 AND archived = false;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        model Task {
          id        Int      @id @default(autoincrement())
          title     String?  @db.VarChar(255)
          priority  Int?
          completed Boolean? @default(false)
          archived  Boolean? @default(false)

          @@index([title], where: "completed = false")
          @@index([priority], where: "priority > 3 AND archived = false")
        }
    "#};

    let expectation = expect![[r#"
        model Task {
          id        Int      @id @default(autoincrement())
          title     String?  @db.VarChar(255)
          priority  Int?
          completed Boolean? @default(false)
          archived  Boolean? @default(false)

          @@index([title], where: "(completed = false)")
          @@index([priority], where: "((priority > 3) AND (archived = false))")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;
    Ok(())
}
