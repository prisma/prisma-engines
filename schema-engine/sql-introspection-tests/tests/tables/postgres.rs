use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
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
          provider = "postgresql"
        }

        model stringstest {
          id             Int    @id
          needs_escaping String @default("\nabc def\nbackspaces: \\abcd\\\n\t(tab character)\nand \"quotes\" and a vertical tabulation here ->\u0016<-\n\n")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_unique(api: &mut TestApi) -> TestResult {
    let setup = r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE UNIQUE INDEX "A_a_key" ON "A" (a DESC);
   "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int @unique(sort: Desc)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_compound_unique(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE UNIQUE INDEX "A_a_b_key" ON "A" (a ASC, b DESC);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn a_table_with_descending_index(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_b_idx" ON "A" (a ASC, b DESC);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(map: "a_pkey")
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_a_hash_index(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_idx" ON "A" USING HASH (a);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int  @id(map: "a_pkey")
          a  Int?

          @@index([a], type: Hash)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspection_of_partial_indices(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL,
           a  INTEGER,
           CONSTRAINT A_pkey PRIMARY KEY (id)
       );

       CREATE INDEX "A_a_idx" ON "A" Using Btree (a) Where (a is not null);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id Int  @id(map: "a_pkey")
          a  Int?

          @@index([a], where: raw("(a IS NOT NULL)"))
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn introspecting_now_functions(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL Primary Key,
           timestamp Timestamp Default now(),
           timestamp_tz Timestamp with time zone Default now(),
           date date Default now(),
           timestamp_2 Timestamp Default current_timestamp,
           timestamp_tz_2 Timestamp with time zone Default current_timestamp,
           date_2 date Default current_timestamp
        );

       "#};
    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id             Int       @id
          timestamp      DateTime? @default(now()) @db.Timestamp(6)
          timestamp_tz   DateTime? @default(now()) @db.Timestamptz(6)
          date           DateTime? @default(now()) @db.Date
          timestamp_2    DateTime? @default(now()) @db.Timestamp(6)
          timestamp_tz_2 DateTime? @default(now()) @db.Timestamptz(6)
          date_2         DateTime? @default(now()) @db.Date
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// https://github.com/prisma/prisma/issues/12095
#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_json_columns(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar" JSONB DEFAULT '{"message": "This message includes a quote: Here''s it!"}',

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Foo {
          id  Int   @id
          bar Json? @default("{\"message\": \"This message includes a quote: Here's it!\"}")
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn datetime_default_expressions_are_not_truncated(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            trial_expires TIMESTAMPTZ(6) NOT NULL DEFAULT now()::TIMESTAMPTZ + '14 days'::INTERVAL,

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Foo {
          id            Int      @id
          trial_expires DateTime @default(dbgenerated("(now() + '14 days'::interval)")) @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Postgres12, Postgres14), exclude(CockroachDb))]
async fn northwind(api: TestApi) {
    let setup = include_str!("./northwind_postgresql.sql");
    api.raw_cmd(setup).await;
    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model categories {
          category_id   Int        @id(map: "pk_categories") @db.SmallInt
          category_name String     @db.VarChar(15)
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
          company_name           String                   @db.VarChar(40)
          contact_name           String?                  @db.VarChar(30)
          contact_title          String?                  @db.VarChar(30)
          address                String?                  @db.VarChar(60)
          city                   String?                  @db.VarChar(15)
          region                 String?                  @db.VarChar(15)
          postal_code            String?                  @db.VarChar(10)
          country                String?                  @db.VarChar(15)
          phone                  String?                  @db.VarChar(24)
          fax                    String?                  @db.VarChar(24)
          customer_customer_demo customer_customer_demo[]
          orders                 orders[]
        }

        model employee_territories {
          employee_id  Int         @db.SmallInt
          territory_id String      @db.VarChar(20)
          employees    employees   @relation(fields: [employee_id], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_territories_employees")
          territories  territories @relation(fields: [territory_id], references: [territory_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_territories_territories")

          @@id([employee_id, territory_id], map: "pk_employee_territories")
        }

        model employees {
          employee_id          Int                    @id(map: "pk_employees") @db.SmallInt
          last_name            String                 @db.VarChar(20)
          first_name           String                 @db.VarChar(10)
          title                String?                @db.VarChar(30)
          title_of_courtesy    String?                @db.VarChar(25)
          birth_date           DateTime?              @db.Date
          hire_date            DateTime?              @db.Date
          address              String?                @db.VarChar(60)
          city                 String?                @db.VarChar(15)
          region               String?                @db.VarChar(15)
          postal_code          String?                @db.VarChar(10)
          country              String?                @db.VarChar(15)
          home_phone           String?                @db.VarChar(24)
          extension            String?                @db.VarChar(4)
          photo                Bytes?
          notes                String?
          reports_to           Int?                   @db.SmallInt
          photo_path           String?                @db.VarChar(255)
          employee_territories employee_territories[]
          employees            employees?             @relation("employeesToemployees", fields: [reports_to], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employees_employees")
          other_employees      employees[]            @relation("employeesToemployees")
          orders               orders[]
        }

        model order_details {
          order_id   Int      @db.SmallInt
          product_id Int      @db.SmallInt
          unit_price Float    @db.Real
          quantity   Int      @db.SmallInt
          discount   Float    @db.Real
          orders     orders   @relation(fields: [order_id], references: [order_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_orders")
          products   products @relation(fields: [product_id], references: [product_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_products")

          @@id([order_id, product_id], map: "pk_order_details")
        }

        model orders {
          order_id         Int             @id(map: "pk_orders") @db.SmallInt
          customer_id      String?         @db.Char
          employee_id      Int?            @db.SmallInt
          order_date       DateTime?       @db.Date
          required_date    DateTime?       @db.Date
          shipped_date     DateTime?       @db.Date
          ship_via         Int?            @db.SmallInt
          freight          Float?          @db.Real
          ship_name        String?         @db.VarChar(40)
          ship_address     String?         @db.VarChar(60)
          ship_city        String?         @db.VarChar(15)
          ship_region      String?         @db.VarChar(15)
          ship_postal_code String?         @db.VarChar(10)
          ship_country     String?         @db.VarChar(15)
          order_details    order_details[]
          customers        customers?      @relation(fields: [customer_id], references: [customer_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_customers")
          employees        employees?      @relation(fields: [employee_id], references: [employee_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_employees")
          shippers         shippers?       @relation(fields: [ship_via], references: [shipper_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_shippers")
        }

        model products {
          product_id        Int             @id(map: "pk_products") @db.SmallInt
          product_name      String          @db.VarChar(40)
          supplier_id       Int?            @db.SmallInt
          category_id       Int?            @db.SmallInt
          quantity_per_unit String?         @db.VarChar(20)
          unit_price        Float?          @db.Real
          units_in_stock    Int?            @db.SmallInt
          units_on_order    Int?            @db.SmallInt
          reorder_level     Int?            @db.SmallInt
          discontinued      Int
          order_details     order_details[]
          categories        categories?     @relation(fields: [category_id], references: [category_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_products_categories")
          suppliers         suppliers?      @relation(fields: [supplier_id], references: [supplier_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_products_suppliers")
        }

        model region {
          region_id          Int           @id(map: "pk_region") @db.SmallInt
          region_description String        @db.Char
          territories        territories[]
        }

        model shippers {
          shipper_id   Int      @id(map: "pk_shippers") @db.SmallInt
          company_name String   @db.VarChar(40)
          phone        String?  @db.VarChar(24)
          orders       orders[]
        }

        model suppliers {
          supplier_id   Int        @id(map: "pk_suppliers") @db.SmallInt
          company_name  String     @db.VarChar(40)
          contact_name  String?    @db.VarChar(30)
          contact_title String?    @db.VarChar(30)
          address       String?    @db.VarChar(60)
          city          String?    @db.VarChar(15)
          region        String?    @db.VarChar(15)
          postal_code   String?    @db.VarChar(10)
          country       String?    @db.VarChar(15)
          phone         String?    @db.VarChar(24)
          fax           String?    @db.VarChar(24)
          homepage      String?
          products      products[]
        }

        model territories {
          territory_id          String                 @id(map: "pk_territories") @db.VarChar(20)
          territory_description String                 @db.Char
          region_id             Int                    @db.SmallInt
          employee_territories  employee_territories[]
          region                region                 @relation(fields: [region_id], references: [region_id], onDelete: NoAction, onUpdate: NoAction, map: "fk_territories_region")
        }

        model us_states {
          state_id     Int     @id(map: "pk_usstates") @db.SmallInt
          state_name   String? @db.VarChar(100)
          state_abbr   String? @db.VarChar(2)
          state_region String? @db.VarChar(50)
        }
    "#]];
    api.expect_datamodel(&expectation).await;
}
