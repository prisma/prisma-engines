use indoc::{formatdoc, indoc};
use introspection_connector::{IntrospectionConnector, IntrospectionContext};
use introspection_engine_tests::test_api::*;
use psl::dml::Datamodel;
use sql_introspection_connector::SqlIntrospectionConnector;
use url::Url;

#[test_connector(tags(Mysql))]
async fn a_table_with_non_id_autoincrement(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE `Test` (
            `id` INTEGER PRIMARY KEY,
            `authorId` INTEGER AUTO_INCREMENT UNIQUE
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id       Int @id
          authorId Int @unique(map: "authorId") @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_length_prefixed_primary_key(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` TEXT NOT NULL,
            CONSTRAINT A_id_pkey PRIMARY KEY (id(30))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id String @id(length: 30) @db.Text
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_length_prefixed_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            CONSTRAINT A_a_key UNIQUE (a(30))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @unique(length: 30) @db.Text
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_length_prefixed_compound_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            `b`  TEXT NOT NULL,
            CONSTRAINT A_a_b_key UNIQUE (a(30), b(20))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@unique([a(length: 30), b(length: 20)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_length_prefixed_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            `b`  TEXT NOT NULL
        );
        
        CREATE INDEX A_a_b_idx ON `A` (a(30), b(20));
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@index([a(length: 30), b(length: 20)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_non_length_prefixed_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  VARCHAR(190) NOT NULL,
            `b`  VARCHAR(192) NOT NULL
        );
        
        CREATE INDEX A_a_idx ON `A` (a);
        CREATE INDEX A_b_idx ON `A` (b(191));
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(190)
          b  String @db.VarChar(192)

          @@index([a])
          @@index([b(length: 191)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_descending_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  INT NOT NULL,
            `b`  INT NOT NULL
        );
        
        CREATE INDEX A_a_b_idx ON `A` (a ASC, b DESC);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int @id
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_descending_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  INT NOT NULL,
            `b`  INT NOT NULL
        );
        
        CREATE UNIQUE INDEX A_a_b_key ON `A` (a ASC, b DESC);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int @id
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("fullTextIndex"))]
async fn a_table_with_fulltext_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );
        
        CREATE FULLTEXT INDEX A_a_b_idx ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@fulltext([a, b])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("fullTextIndex"))]
async fn a_table_with_fulltext_index_with_custom_name(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );
        
        CREATE FULLTEXT INDEX custom_name ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@fulltext([a, b], map: "custom_name")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_table_with_fulltext_index_without_preview_flag(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );

        CREATE FULLTEXT INDEX A_a_b_idx ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@index([a, b])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Mariadb))]
async fn date_time_defaults(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            d1 DATE DEFAULT '2020-01-01',
            d2 DATETIME DEFAULT '2038-01-19 03:14:08',
            d3 TIME DEFAULT '16:20:00'
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("'2020-01-01'")) @db.Date
          d2 DateTime? @default(dbgenerated("'2038-01-19 03:14:08'")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("'16:20:00'")) @db.Time(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mariadb))]
async fn date_time_defaults_mariadb(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            d1 DATE DEFAULT '2020-01-01',
            d2 DATETIME DEFAULT '2038-01-19 03:14:08',
            d3 TIME DEFAULT '16:20:00'
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("('2020-01-01')")) @db.Date
          d2 DateTime? @default(dbgenerated("('2038-01-19 03:14:08')")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("('16:20:00')")) @db.Time(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
async fn missing_select_rights(api: &TestApi) -> TestResult {
    let setup = formatdoc!(
        r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            val INT NOT NULL,
            data VARCHAR(20) NULL
        );

        CREATE INDEX `test_index` ON `A` (`data`);
        CREATE UNIQUE INDEX `test_unique` ON `A` (`val`);

        DROP USER IF EXISTS 'jeffrey'@'%';
        CREATE USER 'jeffrey'@'%' IDENTIFIED BY 'password';
        GRANT USAGE, CREATE ON TABLE `{}`.* TO 'jeffrey'@'%';
        FLUSH PRIVILEGES;
    "#,
        api.schema_name()
    );

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          val  Int     @unique(map: "test_unique")
          data String? @db.VarChar(20)

          @@index([data], map: "test_index")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    let mut url: Url = api.connection_string().parse()?;
    url.set_username("jeffrey").unwrap();
    url.set_password(Some("password")).unwrap();

    let conn = SqlIntrospectionConnector::new(url.as_ref(), Default::default()).await?;

    let datasource = formatdoc!(
        r#"
        datasource db {{
          provider = "mysql"
          url      = "{url}"
        }}
    "#
    );

    let config = psl::parse_configuration(&datasource).unwrap();

    let ctx = IntrospectionContext {
        source: config.datasources.into_iter().next().unwrap(),
        composite_type_depth: Default::default(),
        preview_features: Default::default(),
    };

    let res = conn.introspect(&Datamodel::new(), ctx).await.unwrap();
    assert!(res.data_model.is_empty());

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn northwind(api: TestApi) {
    let setup = include_str!("./northwind_mysql.sql");
    api.raw_cmd(setup).await;
    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model customers {
          id              Int      @id @default(autoincrement())
          company         String?  @db.VarChar(50)
          last_name       String?  @db.VarChar(50)
          first_name      String?  @db.VarChar(50)
          email_address   String?  @db.VarChar(50)
          job_title       String?  @db.VarChar(50)
          business_phone  String?  @db.VarChar(25)
          home_phone      String?  @db.VarChar(25)
          mobile_phone    String?  @db.VarChar(25)
          fax_number      String?  @db.VarChar(25)
          address         String?  @db.LongText
          city            String?  @db.VarChar(50)
          state_province  String?  @db.VarChar(50)
          zip_postal_code String?  @db.VarChar(15)
          country_region  String?  @db.VarChar(50)
          web_page        String?  @db.LongText
          notes           String?  @db.LongText
          attachments     Bytes?
          orders          orders[]

          @@index([city], map: "city")
          @@index([company], map: "company")
          @@index([first_name], map: "first_name")
          @@index([last_name], map: "last_name")
          @@index([state_province], map: "state_province")
          @@index([zip_postal_code], map: "zip_postal_code")
        }

        model employee_privileges {
          employee_id  Int
          privilege_id Int
          employees    employees  @relation(fields: [employee_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_privileges_employees1")
          privileges   privileges @relation(fields: [privilege_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_employee_privileges_privileges1")

          @@id([employee_id, privilege_id])
          @@index([employee_id], map: "employee_id")
          @@index([privilege_id], map: "privilege_id")
          @@index([privilege_id], map: "privilege_id_2")
        }

        model employees {
          id                  Int                   @id @default(autoincrement())
          company             String?               @db.VarChar(50)
          last_name           String?               @db.VarChar(50)
          first_name          String?               @db.VarChar(50)
          email_address       String?               @db.VarChar(50)
          job_title           String?               @db.VarChar(50)
          business_phone      String?               @db.VarChar(25)
          home_phone          String?               @db.VarChar(25)
          mobile_phone        String?               @db.VarChar(25)
          fax_number          String?               @db.VarChar(25)
          address             String?               @db.LongText
          city                String?               @db.VarChar(50)
          state_province      String?               @db.VarChar(50)
          zip_postal_code     String?               @db.VarChar(15)
          country_region      String?               @db.VarChar(50)
          web_page            String?               @db.LongText
          notes               String?               @db.LongText
          attachments         Bytes?
          employee_privileges employee_privileges[]
          orders              orders[]
          purchase_orders     purchase_orders[]

          @@index([city], map: "city")
          @@index([company], map: "company")
          @@index([first_name], map: "first_name")
          @@index([last_name], map: "last_name")
          @@index([state_province], map: "state_province")
          @@index([zip_postal_code], map: "zip_postal_code")
        }

        model inventory_transaction_types {
          id                     Int                      @id @db.TinyInt
          type_name              String                   @db.VarChar(50)
          inventory_transactions inventory_transactions[]
        }

        model inventory_transactions {
          id                          Int                         @id @default(autoincrement())
          transaction_type            Int                         @db.TinyInt
          transaction_created_date    DateTime?                   @db.DateTime(0)
          transaction_modified_date   DateTime?                   @db.DateTime(0)
          product_id                  Int
          quantity                    Int
          purchase_order_id           Int?
          customer_order_id           Int?
          comments                    String?                     @db.VarChar(255)
          inventory_transaction_types inventory_transaction_types @relation(fields: [transaction_type], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_inventory_transactions_inventory_transaction_types1")
          orders                      orders?                     @relation(fields: [customer_order_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_inventory_transactions_orders1")
          products                    products                    @relation(fields: [product_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_inventory_transactions_products1")
          purchase_orders             purchase_orders?            @relation(fields: [purchase_order_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_inventory_transactions_purchase_orders1")
          purchase_order_details      purchase_order_details[]

          @@index([customer_order_id], map: "customer_order_id")
          @@index([customer_order_id], map: "customer_order_id_2")
          @@index([product_id], map: "product_id")
          @@index([product_id], map: "product_id_2")
          @@index([purchase_order_id], map: "purchase_order_id")
          @@index([purchase_order_id], map: "purchase_order_id_2")
          @@index([transaction_type], map: "transaction_type")
        }

        model invoices {
          id           Int       @id @default(autoincrement())
          order_id     Int?
          invoice_date DateTime? @db.DateTime(0)
          due_date     DateTime? @db.DateTime(0)
          tax          Decimal?  @default(0.0000) @db.Decimal(19, 4)
          shipping     Decimal?  @default(0.0000) @db.Decimal(19, 4)
          amount_due   Decimal?  @default(0.0000) @db.Decimal(19, 4)
          orders       orders?   @relation(fields: [order_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_invoices_orders1")

          @@index([order_id], map: "fk_invoices_orders1_idx")
          @@index([id], map: "id")
          @@index([id], map: "id_2")
        }

        model order_details {
          id                   Int                   @id @default(autoincrement())
          order_id             Int
          product_id           Int?
          quantity             Decimal               @default(0.0000) @db.Decimal(18, 4)
          unit_price           Decimal?              @default(0.0000) @db.Decimal(19, 4)
          discount             Float                 @default(0)
          status_id            Int?
          date_allocated       DateTime?             @db.DateTime(0)
          purchase_order_id    Int?
          inventory_id         Int?
          order_details_status order_details_status? @relation(fields: [status_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_order_details_status1")
          orders               orders                @relation(fields: [order_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_orders1")
          products             products?             @relation(fields: [product_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_order_details_products1")

          @@index([status_id], map: "fk_order_details_order_details_status1_idx")
          @@index([order_id], map: "fk_order_details_orders1_idx")
          @@index([id], map: "id")
          @@index([id], map: "id_2")
          @@index([id], map: "id_3")
          @@index([id], map: "id_4")
          @@index([id], map: "id_5")
          @@index([inventory_id], map: "inventory_id")
          @@index([product_id], map: "product_id")
          @@index([product_id], map: "product_id_2")
          @@index([purchase_order_id], map: "purchase_order_id")
        }

        model order_details_status {
          id            Int             @id
          status_name   String          @db.VarChar(50)
          order_details order_details[]
        }

        model orders {
          id                     Int                      @id @default(autoincrement())
          employee_id            Int?
          customer_id            Int?
          order_date             DateTime?                @db.DateTime(0)
          shipped_date           DateTime?                @db.DateTime(0)
          shipper_id             Int?
          ship_name              String?                  @db.VarChar(50)
          ship_address           String?                  @db.LongText
          ship_city              String?                  @db.VarChar(50)
          ship_state_province    String?                  @db.VarChar(50)
          ship_zip_postal_code   String?                  @db.VarChar(50)
          ship_country_region    String?                  @db.VarChar(50)
          shipping_fee           Decimal?                 @default(0.0000) @db.Decimal(19, 4)
          taxes                  Decimal?                 @default(0.0000) @db.Decimal(19, 4)
          payment_type           String?                  @db.VarChar(50)
          paid_date              DateTime?                @db.DateTime(0)
          notes                  String?                  @db.LongText
          tax_rate               Float?                   @default(0)
          tax_status_id          Int?                     @db.TinyInt
          status_id              Int?                     @default(0) @db.TinyInt
          customers              customers?               @relation(fields: [customer_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_customers")
          employees              employees?               @relation(fields: [employee_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_employees1")
          orders_status          orders_status?           @relation(fields: [status_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_orders_status1")
          orders_tax_status      orders_tax_status?       @relation(fields: [tax_status_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_orders_tax_status1")
          shippers               shippers?                @relation(fields: [shipper_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_orders_shippers1")
          inventory_transactions inventory_transactions[]
          invoices               invoices[]
          order_details          order_details[]

          @@index([customer_id], map: "customer_id")
          @@index([customer_id], map: "customer_id_2")
          @@index([employee_id], map: "employee_id")
          @@index([employee_id], map: "employee_id_2")
          @@index([status_id], map: "fk_orders_orders_status1")
          @@index([id], map: "id")
          @@index([id], map: "id_2")
          @@index([id], map: "id_3")
          @@index([ship_zip_postal_code], map: "ship_zip_postal_code")
          @@index([shipper_id], map: "shipper_id")
          @@index([shipper_id], map: "shipper_id_2")
          @@index([tax_status_id], map: "tax_status")
        }

        model orders_status {
          id          Int      @id @db.TinyInt
          status_name String   @db.VarChar(50)
          orders      orders[]
        }

        model orders_tax_status {
          id              Int      @id @db.TinyInt
          tax_status_name String   @db.VarChar(50)
          orders          orders[]
        }

        model privileges {
          id                  Int                   @id @default(autoincrement())
          privilege_name      String?               @db.VarChar(50)
          employee_privileges employee_privileges[]
        }

        model products {
          supplier_ids             String?                  @db.LongText
          id                       Int                      @id @default(autoincrement())
          product_code             String?                  @db.VarChar(25)
          product_name             String?                  @db.VarChar(50)
          description              String?                  @db.LongText
          standard_cost            Decimal?                 @default(0.0000) @db.Decimal(19, 4)
          list_price               Decimal                  @default(0.0000) @db.Decimal(19, 4)
          reorder_level            Int?
          target_level             Int?
          quantity_per_unit        String?                  @db.VarChar(50)
          discontinued             Boolean                  @default(false)
          minimum_reorder_quantity Int?
          category                 String?                  @db.VarChar(50)
          attachments              Bytes?
          inventory_transactions   inventory_transactions[]
          order_details            order_details[]
          purchase_order_details   purchase_order_details[]

          @@index([product_code], map: "product_code")
        }

        model purchase_order_details {
          id                     Int                     @id @default(autoincrement())
          purchase_order_id      Int
          product_id             Int?
          quantity               Decimal                 @db.Decimal(18, 4)
          unit_cost              Decimal                 @db.Decimal(19, 4)
          date_received          DateTime?               @db.DateTime(0)
          posted_to_inventory    Boolean                 @default(false)
          inventory_id           Int?
          inventory_transactions inventory_transactions? @relation(fields: [inventory_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_order_details_inventory_transactions1")
          products               products?               @relation(fields: [product_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_order_details_products1")
          purchase_orders        purchase_orders         @relation(fields: [purchase_order_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_order_details_purchase_orders1")

          @@index([id], map: "id")
          @@index([inventory_id], map: "inventory_id")
          @@index([inventory_id], map: "inventory_id_2")
          @@index([product_id], map: "product_id")
          @@index([product_id], map: "product_id_2")
          @@index([purchase_order_id], map: "purchase_order_id")
          @@index([purchase_order_id], map: "purchase_order_id_2")
        }

        model purchase_order_status {
          id              Int               @id
          status          String?           @db.VarChar(50)
          purchase_orders purchase_orders[]
        }

        model purchase_orders {
          id                     Int                      @id @unique(map: "id") @default(autoincrement())
          supplier_id            Int?
          created_by             Int?
          submitted_date         DateTime?                @db.DateTime(0)
          creation_date          DateTime?                @db.DateTime(0)
          status_id              Int?                     @default(0)
          expected_date          DateTime?                @db.DateTime(0)
          shipping_fee           Decimal                  @default(0.0000) @db.Decimal(19, 4)
          taxes                  Decimal                  @default(0.0000) @db.Decimal(19, 4)
          payment_date           DateTime?                @db.DateTime(0)
          payment_amount         Decimal?                 @default(0.0000) @db.Decimal(19, 4)
          payment_method         String?                  @db.VarChar(50)
          notes                  String?                  @db.LongText
          approved_by            Int?
          approved_date          DateTime?                @db.DateTime(0)
          submitted_by           Int?
          employees              employees?               @relation(fields: [created_by], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_orders_employees1")
          purchase_order_status  purchase_order_status?   @relation(fields: [status_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_orders_purchase_order_status1")
          suppliers              suppliers?               @relation(fields: [supplier_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "fk_purchase_orders_suppliers1")
          inventory_transactions inventory_transactions[]
          purchase_order_details purchase_order_details[]

          @@index([created_by], map: "created_by")
          @@index([id], map: "id_2")
          @@index([status_id], map: "status_id")
          @@index([supplier_id], map: "supplier_id")
          @@index([supplier_id], map: "supplier_id_2")
        }

        model sales_reports {
          group_by          String  @id @db.VarChar(50)
          display           String? @db.VarChar(50)
          title             String? @db.VarChar(50)
          filter_row_source String? @db.LongText
          default           Boolean @default(false)
        }

        model shippers {
          id              Int      @id @default(autoincrement())
          company         String?  @db.VarChar(50)
          last_name       String?  @db.VarChar(50)
          first_name      String?  @db.VarChar(50)
          email_address   String?  @db.VarChar(50)
          job_title       String?  @db.VarChar(50)
          business_phone  String?  @db.VarChar(25)
          home_phone      String?  @db.VarChar(25)
          mobile_phone    String?  @db.VarChar(25)
          fax_number      String?  @db.VarChar(25)
          address         String?  @db.LongText
          city            String?  @db.VarChar(50)
          state_province  String?  @db.VarChar(50)
          zip_postal_code String?  @db.VarChar(15)
          country_region  String?  @db.VarChar(50)
          web_page        String?  @db.LongText
          notes           String?  @db.LongText
          attachments     Bytes?
          orders          orders[]

          @@index([city], map: "city")
          @@index([company], map: "company")
          @@index([first_name], map: "first_name")
          @@index([last_name], map: "last_name")
          @@index([state_province], map: "state_province")
          @@index([zip_postal_code], map: "zip_postal_code")
        }

        model strings {
          string_id   Int     @id @default(autoincrement())
          string_data String? @db.VarChar(255)
        }

        model suppliers {
          id              Int               @id @default(autoincrement())
          company         String?           @db.VarChar(50)
          last_name       String?           @db.VarChar(50)
          first_name      String?           @db.VarChar(50)
          email_address   String?           @db.VarChar(50)
          job_title       String?           @db.VarChar(50)
          business_phone  String?           @db.VarChar(25)
          home_phone      String?           @db.VarChar(25)
          mobile_phone    String?           @db.VarChar(25)
          fax_number      String?           @db.VarChar(25)
          address         String?           @db.LongText
          city            String?           @db.VarChar(50)
          state_province  String?           @db.VarChar(50)
          zip_postal_code String?           @db.VarChar(15)
          country_region  String?           @db.VarChar(50)
          web_page        String?           @db.LongText
          notes           String?           @db.LongText
          attachments     Bytes?
          purchase_orders purchase_orders[]

          @@index([city], map: "city")
          @@index([company], map: "company")
          @@index([first_name], map: "first_name")
          @@index([last_name], map: "last_name")
          @@index([state_province], map: "state_province")
          @@index([zip_postal_code], map: "zip_postal_code")
        }
    "#]];
    api.expect_datamodel(&expectation).await;
}
