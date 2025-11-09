use barrel::types;
use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn negative_default_values_should_work(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Blog" (
            id          SERIAL PRIMARY KEY,
            int         INT4 NOT NULL DEFAULT 1,
            neg_int     INT4 NOT NULL DEFAULT -1,
            float       FLOAT4 NOT NULL DEFAULT 2.1,
            neg_float   FLOAT4 NOT NULL DEFAULT -2.1,
            bigint      INT8 NOT NULL DEFAULT 3,
            neg_bigint  INT8 NOT NULL DEFAULT -3
        )
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Blog {
          id         BigInt @id @default(autoincrement())
          int        Int    @default(1)
          neg_int    Int    @default(-1)
          float      Float  @default(2.1) @db.Float4
          neg_float  Float  @default(-2.1) @db.Float4
          bigint     BigInt @default(3)
          neg_bigint BigInt @default(-3)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn should_ignore_prisma_helper_tables(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Blog" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "_RelayId" (
            id SERIAL PRIMARY KEY,
            stablemodelidentifier STRING NOT NULL
        );

        CREATE TABLE "_Migration" (
            revision STRING NOT NULL,
            name STRING NOT NULL,
            datamodel STRING NOT NULL,
            status STRING NOT NULL,
            applied STRING NOT NULL,
            rolled_back STRING NOT NULL,
            datamodel_steps STRING NOT NULL,
            database_migration STRING NOT NULL,
            errors STRING NOT NULL,
            started_at STRING NOT NULL,
            finished_at STRING NOT NULL
        );

        CREATE TABLE "_prisma_migrations" (
            id SERIAL PRIMARY KEY,
            checksum STRING NOT NULL,
            finished_at STRING,
            migration_name STRING,
            logs STRING,
            rolled_back_at STRING,
            started_at STRING NOT NULL,
            applied_steps_count INT4
        );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Blog {
          id BigInt @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn default_values(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Test" (
            id SERIAL PRIMARY KEY,
            string_static_char CHAR(5) DEFAULT 'test',
            string_static_char_null CHAR(5) DEFAULT NULL,
            string_static_varchar VARCHAR(5) DEFAULT 'test',
            int_static INT4 DEFAULT 2,
            float_static FLOAT4 DEFAULT 1.43,
            boolean_static BOOL DEFAULT true,
            datetime_now TIMESTAMPTZ DEFAULT current_timestamp()
        );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Test {
          id                      BigInt    @id @default(autoincrement())
          string_static_char      String?   @default("test") @db.Char(5)
          string_static_char_null String?   @db.Char(5)
          string_static_varchar   String?   @default("test") @db.String(5)
          int_static              Int?      @default(2)
          float_static            Float?    @default(1.43) @db.Float4
          boolean_static          Boolean?  @default(true)
          datetime_now            DateTime? @default(now()) @db.Timestamptz(6)
        }
    "#]];
    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn a_simple_table_with_gql_types(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::datetime());
                t.add_column("id", types::integer().increments(true));
                t.add_column("int", types::integer());
                t.add_column("string", types::text());

                t.add_constraint("Blog_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Blog {
          bool   Boolean
          float  Float
          date   DateTime @db.Timestamp(6)
          id     BigInt   @id @default(autoincrement())
          int    Int
          string String
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn introspecting_a_table_with_json_type_must_work_cockroach(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("json", types::json());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Blog {
            id      BigInt @id @default(autoincrement())
            json    Json
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

// Cockroach can return non-deterministic results if the UNIQUE constraint is defined twice
// (it does not collapse similar unique constraints). This variation does not include the
// doubly defined unique constraint.
#[test_connector(tags(CockroachDb))]
async fn a_table_with_non_id_autoincrement_cockroach(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::serial().unique(true));

                t.add_constraint("Test_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int @id
            authorId BigInt @default(autoincrement()) @unique
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn introspecting_json_defaults_on_cockroach(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL PRIMARY KEY,
           json JSON DEFAULT '[]'::json,
           jsonb JSONB DEFAULT '{}'::jsonb,
           jsonb_string JSONB DEFAULT E'"ab\'c"',
           jsonb_object JSONB DEFAULT '{"a": ["b''"], "c": true, "d": null }'
         );

       "#};
    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id           Int   @id
          json         Json? @default("[]")
          jsonb        Json? @default("{}")
          jsonb_string Json? @default("\"ab'c\"")
          jsonb_object Json? @default("{\"a\": [\"b'\"], \"c\": true, \"d\": null}")
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn string_defaults_that_need_escaping(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "stringstest" (
            id INTEGER PRIMARY KEY,
            needs_escaping TEXT NOT NULL DEFAULT $$
abc def
backspaces: \abcd\
	(tab character)
and "quotes" and a vertical tabulation here -><-

$$
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model stringstest {
          id             Int    @id
          needs_escaping String @default("\nabc def\nbackspaces: \\abcd\\\n\t(tab character)\nand \"quotes\" and a vertical tabulation here ->x16<-\n\n")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn datetime_default_expressions_are_not_truncated(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            trial_expires TIMESTAMPTZ(6) NOT NULL DEFAULT now():::TIMESTAMPTZ + '14 days':::INTERVAL,

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Foo {
          id            Int      @id
          trial_expires DateTime @default(dbgenerated("now() + '14 days'::INTERVAL")) @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn northwind(api: TestApi) {
    let setup = include_str!("./northwind_postgresql.sql");
    api.raw_cmd(setup).await;
    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model categories {
          category_id   Int        @id(map: "pk_categories") @db.Int2
          category_name String     @db.String(15)
          description   String?
          picture       Bytes?
          products      products[]
        }

        model customer_customer_demo {
          customer_id           String                @db.Char
          customer_type_id      String                @db.Char
          customer_demographics customer_demographics @relation(fields: [customer_type_id], references: [customer_type_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_customer_customer_demo_customer_demographics")
          customers             customers             @relation(fields: [customer_id], references: [customer_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_customer_customer_demo_customers")

          @@id([customer_id, customer_type_id], map: "pk_customer_customer_demo")
        }

        model customer_demographics {
          customer_type_id       String                   @id(map: "pk_customer_demographics") @db.Char
          customer_desc          String?
          customer_customer_demo customer_customer_demo[]
        }

        model customers {
          customer_id            String                   @id(map: "pk_customers") @db.Char
          company_name           String                   @db.String(40)
          contact_name           String?                  @db.String(30)
          contact_title          String?                  @db.String(30)
          address                String?                  @db.String(60)
          city                   String?                  @db.String(15)
          region                 String?                  @db.String(15)
          postal_code            String?                  @db.String(10)
          country                String?                  @db.String(15)
          phone                  String?                  @db.String(24)
          fax                    String?                  @db.String(24)
          customer_customer_demo customer_customer_demo[]
          orders                 orders[]
        }

        model employee_territories {
          employee_id  Int         @db.Int2
          territory_id String      @db.String(20)
          employees    employees   @relation(fields: [employee_id], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_territories_employees")
          territories  territories @relation(fields: [territory_id], references: [territory_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_territories_territories")

          @@id([employee_id, territory_id], map: "pk_employee_territories")
        }

        model employees {
          employee_id          Int                    @id(map: "pk_employees") @db.Int2
          last_name            String                 @db.String(20)
          first_name           String                 @db.String(10)
          title                String?                @db.String(30)
          title_of_courtesy    String?                @db.String(25)
          birth_date           DateTime?              @db.Date
          hire_date            DateTime?              @db.Date
          address              String?                @db.String(60)
          city                 String?                @db.String(15)
          region               String?                @db.String(15)
          postal_code          String?                @db.String(10)
          country              String?                @db.String(15)
          home_phone           String?                @db.String(24)
          extension            String?                @db.String(4)
          photo                Bytes?
          notes                String?
          reports_to           Int?                   @db.Int2
          photo_path           String?                @db.String(255)
          employee_territories employee_territories[]
          employees            employees?             @relation("employeesToemployees", fields: [reports_to], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employees_employees")
          other_employees      employees[]            @relation("employeesToemployees")
          orders               orders[]
        }

        model order_details {
          order_id   Int      @db.Int2
          product_id Int      @db.Int2
          unit_price Float    @db.Float4
          quantity   Int      @db.Int2
          discount   Float    @db.Float4
          orders     orders   @relation(fields: [order_id], references: [order_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_orders")
          products   products @relation(fields: [product_id], references: [product_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_products")

          @@id([order_id, product_id], map: "pk_order_details")
        }

        model orders {
          order_id         Int             @id(map: "pk_orders") @db.Int2
          customer_id      String?         @db.Char
          employee_id      Int?            @db.Int2
          order_date       DateTime?       @db.Date
          required_date    DateTime?       @db.Date
          shipped_date     DateTime?       @db.Date
          ship_via         Int?            @db.Int2
          freight          Float?          @db.Float4
          ship_name        String?         @db.String(40)
          ship_address     String?         @db.String(60)
          ship_city        String?         @db.String(15)
          ship_region      String?         @db.String(15)
          ship_postal_code String?         @db.String(10)
          ship_country     String?         @db.String(15)
          order_details    order_details[]
          customers        customers?      @relation(fields: [customer_id], references: [customer_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_customers")
          employees        employees?      @relation(fields: [employee_id], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_employees")
          shippers         shippers?       @relation(fields: [ship_via], references: [shipper_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_shippers")
        }

        model products {
          product_id        Int             @id(map: "pk_products") @db.Int2
          product_name      String          @db.String(40)
          supplier_id       Int?            @db.Int2
          category_id       Int?            @db.Int2
          quantity_per_unit String?         @db.String(20)
          unit_price        Float?          @db.Float4
          units_in_stock    Int?            @db.Int2
          units_on_order    Int?            @db.Int2
          reorder_level     Int?            @db.Int2
          discontinued      Int
          order_details     order_details[]
          categories        categories?     @relation(fields: [category_id], references: [category_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_products_categories")
          suppliers         suppliers?      @relation(fields: [supplier_id], references: [supplier_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_products_suppliers")
        }

        model region {
          region_id          Int           @id(map: "pk_region") @db.Int2
          region_description String        @db.Char
          territories        territories[]
        }

        model shippers {
          shipper_id   Int      @id(map: "pk_shippers") @db.Int2
          company_name String   @db.String(40)
          phone        String?  @db.String(24)
          orders       orders[]
        }

        model suppliers {
          supplier_id   Int        @id(map: "pk_suppliers") @db.Int2
          company_name  String     @db.String(40)
          contact_name  String?    @db.String(30)
          contact_title String?    @db.String(30)
          address       String?    @db.String(60)
          city          String?    @db.String(15)
          region        String?    @db.String(15)
          postal_code   String?    @db.String(10)
          country       String?    @db.String(15)
          phone         String?    @db.String(24)
          fax           String?    @db.String(24)
          homepage      String?
          products      products[]
        }

        model territories {
          territory_id          String                 @id(map: "pk_territories") @db.String(20)
          territory_description String                 @db.Char
          region_id             Int                    @db.Int2
          employee_territories  employee_territories[]
          region                region                 @relation(fields: [region_id], references: [region_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_territories_region")
        }

        model us_states {
          state_id     Int     @id(map: "pk_usstates") @db.Int2
          state_name   String? @db.String(100)
          state_abbr   String? @db.String(2)
          state_region String? @db.String(50)
        }
    "#]];
    api.expect_datamodel(&expectation).await;
}
