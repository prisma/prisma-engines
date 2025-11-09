use barrel::types;
use expect_test::expect;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn kanjis(api: &mut TestApi) -> TestResult {
    let migration = indoc! {r#"
        CREATE TABLE "A"
        (
            id  int primary key,
            b患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患 int not null
        );

        CREATE TABLE "B"
        (
            a者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者 int primary key
        );

        ALTER TABLE "A" ADD CONSTRAINT "患者ID" FOREIGN KEY (b患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患患) REFERENCES "B"(a者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者者) ON DELETE RESTRICT ON UPDATE CASCADE;
    "#};

    api.database().raw_cmd(migration).await?;

    let expected = expect![[r#"
        model A {
          id                    Int @id
          b____________________ Int @map("b患患患患患患患患患患患患患患患患患患患患")
          B                     B   @relation(fields: [b____________________], references: [a____________________], map: "患者ID")
        }

        model B {
          a____________________ Int @id @map("a者者者者者者者者者者者者者者者者者者者者")
          A                     A[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
// Cockroach can return either order for multiple foreign keys. This is hard to deterministically
// test, so disable for now. See: https://github.com/cockroachdb/cockroach/issues/71098.
async fn multiple_foreign_key_constraints_are_taken_always_in_the_same_order(api: &mut TestApi) -> TestResult {
    let migration = indoc! {r#"
        CREATE TABLE "A"
        (
            id  int primary key,
            foo int not null
        );

        CREATE TABLE "B"
        (
            id int primary key
        );

        ALTER TABLE "A" ADD CONSTRAINT "fk_1" FOREIGN KEY (foo) REFERENCES "B"(id) ON DELETE CASCADE ON UPDATE CASCADE;
        ALTER TABLE "A" ADD CONSTRAINT "fk_2" FOREIGN KEY (foo) REFERENCES "B"(id) ON DELETE RESTRICT ON UPDATE RESTRICT;
    "#};

    api.database().raw_cmd(migration).await?;

    let expected = expect![[r#"
        model A {
          id  Int @id
          foo Int
          B   B   @relation(fields: [foo], references: [id], onDelete: Cascade, map: "fk_1")
        }

        model B {
          id Int @id
          A  A[]
        }
    "#]];

    for _ in 0..10 {
        expected.assert_eq(&api.introspect_dml().await?);
    }

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn relations_should_avoid_name_clashes_2(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("x", move |t| {
                t.add_column("id", types::primary());
                t.add_column("y", types::integer().nullable(false));
                t.add_index("unique_y_id", types::index(vec!["id", "y"]).unique(true));
            });

            migration.create_table("y", move |t| {
                t.add_column("id", types::primary());
                t.add_column("x", types::integer().nullable(false));
                t.add_column("fk_x_1", types::integer().nullable(false));
                t.add_column("fk_x_2", types::integer().nullable(false));
            });

            migration.change_table("x", |t| {
                t.add_foreign_key(&["y"], "y", &["id"]);
            });

            migration.change_table("y", |t| {
                t.add_constraint(
                    "y_fkey",
                    types::foreign_constraint(&["fk_x_1", "fk_x_2"], "x", &["id", "y"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id                   Int @id @default(autoincrement())
          y                    Int
          y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
          y_y_fk_x_1_fk_x_2Tox y[] @relation("y_fk_x_1_fk_x_2Tox")

          @@unique([id, y], map: "unique_y_id")
        }

        model y {
          id                   Int @id @default(autoincrement())
          x                    Int
          fk_x_1               Int
          fk_x_2               Int
          x_x_yToy             x[] @relation("x_yToy")
          x_y_fk_x_1_fk_x_2Tox x   @relation("y_fk_x_1_fk_x_2Tox", fields: [fk_x_1, fk_x_2], references: [id, y], onDelete: NoAction, onUpdate: NoAction, map: "y_fkey")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn default_values_on_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") Default 0");
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?  @default(0)
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn name_ambiguity_with_a_scalar_field(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "b" (
            id SERIAL PRIMARY KEY,
            a INT NOT NULL
        );

        CREATE TABLE "a" (
            id SERIAL PRIMARY KEY,
            b INT NOT NULL,
            CONSTRAINT "a_b_fkey" FOREIGN KEY (b) REFERENCES "b"(id) ON DELETE RESTRICT ON UPDATE CASCADE
        );
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model a {
          id       Int @id @default(autoincrement())
          b        Int
          b_a_bTob b   @relation("a_bTob", fields: [b], references: [id])
        }

        model b {
          id       Int @id @default(autoincrement())
          a        Int
          a_a_bTob a[] @relation("a_bTob")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn legacy_prisma_many_to_many_relation(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "Post" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "_PostToUser" (
            "A" INT NOT NULL,
            "B" INT NOT NULL,
            CONSTRAINT "_PostToUser_A_fkey" FOREIGN KEY ("A") REFERENCES "Post"(id),
            CONSTRAINT "_PostToUser_B_fkey" FOREIGN KEY ("B") REFERENCES "User"(id)
        );

        CREATE UNIQUE INDEX test ON "_PostToUser" ("A", "B");
        CREATE INDEX test2 ON "_PostToUser" ("B");
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Post {
          id   Int    @id @default(autoincrement())
          User User[]
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn new_prisma_many_to_many_relation(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "Post" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "_PostToUser" (
            "A" INT NOT NULL,
            "B" INT NOT NULL,
            CONSTRAINT "_PostToUser_A_fkey" FOREIGN KEY ("A") REFERENCES "Post"(id),
            CONSTRAINT "_PostToUser_B_fkey" FOREIGN KEY ("B") REFERENCES "User"(id),
            CONSTRAINT "_PostToUser_AB_pkey" PRIMARY KEY ("A", "B")
        );

        CREATE INDEX test ON "_PostToUser" ("B");
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Post {
          id   Int    @id @default(autoincrement())
          User User[]
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
