use datamodel::parse_schema;
use expect_test::{expect, Expect};

fn check(from: &str, to: &str, expectation: Expect) {
    let (ref from_config, ref from_datamodel) = parse_schema(from).unwrap();
    let (ref to_config, ref to_datamodel) = parse_schema(to).unwrap();
    let migration = sql_migration_connector::SqlMigrationConnector::migration_from_schemas(
        (from_config, from_datamodel),
        (to_config, to_datamodel),
    );

    expectation.assert_eq(&migration.drift_summary())
}

#[test]
fn empty_schemas() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        "",
        expect![[""]],
    )
}

#[test]
fn additions_table() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        expect![[r#"

            [+] Added tables
              - Cat
        "#]],
    );
}

#[test]
fn additions_column() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
            name String?
        }
        "#,
        expect![[r#"

            [*] Changed the `Cat` table
              [+] Added column `name`
        "#]],
    );
}

#[test]
fn additions_enum() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        expect![[r#"

            [+] Added enums
              - Color
        "#]],
    );
}

#[test]
fn additions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        expect![[r#"

            [+] Added enums
              - Color

            [+] Added tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_table() {
    check(
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }

        model Cat {
            id Int @id
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "file:test.db"
        }
        "#,
        expect![[r#"

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_enum() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        expect![[r#"

            [-] Removed enums
              - Color
        "#]],
    );
}

#[test]
fn deletions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }
        "#,
        expect![[r#"

            [-] Removed enums
              - Color

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn deletions_column() {
    check(
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://localhost/testdb"
        }

        model Cat {
            id Int @id
            name String?
        }
        "#,
        r#"
        datasource db {
            provider = "sqlite"
            url = "mysql://localhost/testdb"
        }

        model Cat {
            id Int @id
        }
        "#,
        expect![[r#"

            [*] Changed the `Cat` table
              [-] Removed column `name`
        "#]],
    );
}

#[test]
fn additions_and_deletions_mixed() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Dog {
            id Int @id
            name String
        }
        "#,
        expect![[r#"

            [+] Added tables
              - Dog

            [-] Removed enums
              - Color

            [-] Removed tables
              - Cat
        "#]],
    );
}

#[test]
fn multiple_changed_tables_and_enums() {
    check(
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Cat {
            id Int @id
            color Color
        }

        model Dog {
            id Int @id
            name String
            isGoodDog Boolean?
        }

        enum Color {
            RED
            GREEN
            BLUE
        }
        "#,
        r#"
        datasource db {
            provider = "postgres"
            url = "postgres://localhost:5432/testdb"
        }

        model Dog {
            id Int
            weight Int @unique
            isGoodDog Boolean @default(true)
        }

        model Cat {
            id Int @id
            vaccinated Boolean
        }

        enum Color {
            GREEN
            BLUE
            TRANSPARENT
        }

        enum DogType {
            GOOD_DOG
        }

        enum CatType {
            MEOW_MEOW
        }
        "#,
        expect![[r#"

            [+] Added enums
              - DogType
              - CatType

            [*] Changed the `Color` enum
              [+] Added variant `TRANSPARENT`
              [-] Removed variant `RED`

            [*] Changed the `Cat` table
              [-] Removed column `color`
              [+] Added column `vaccinated`

            [*] Changed the `Dog` table
              [-] Dropped the primary key on columns (id)
              [-] Removed column `name`
              [+] Added column `weight`
              [*] Altered column `isGoodDog` (changed from Nullable to Required, default changed from `None` to `Some(Value(Boolean(true)))`)
              [+] Added unique index on columns (weight)
        "#]],
    );
}
