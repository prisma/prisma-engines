use indoc::formatdoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Mssql))]
async fn default_values(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [dbo].[Test] (
            id INTEGER,

            string_static_char CHAR(5) CONSTRAINT [charconstraint] DEFAULT 'test',
            string_static_char_null CHAR(5) CONSTRAINT [charconstraint2] DEFAULT NULL,
            string_static_varchar VARCHAR(5) CONSTRAINT [varcharconstraint] DEFAULT 'test',
            int_static INTEGER CONSTRAINT [intdefault] DEFAULT 2,
            float_static REAL CONSTRAINT [floatdefault] DEFAULT 1.43,
            boolean_static BIT CONSTRAINT [booldefault] DEFAULT 1,
            datetime_now DATETIME CONSTRAINT [datetimedefault] DEFAULT CURRENT_TIMESTAMP,

            CONSTRAINT [Test_pkey] PRIMARY KEY (id)
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id                      Int       @id
          string_static_char      String?   @default("test", map: "charconstraint") @db.Char(5)
          string_static_char_null String?   @db.Char(5)
          string_static_varchar   String?   @default("test", map: "varcharconstraint") @db.VarChar(5)
          int_static              Int?      @default(2, map: "intdefault")
          float_static            Float?    @default(1.43, map: "floatdefault") @db.Real
          boolean_static          Boolean?  @default(true, map: "booldefault")
          datetime_now            DateTime? @default(now(), map: "datetimedefault") @db.DateTime
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn negative_default_values_should_work(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [Blog] (
            id INTEGER IDENTITY,

            int INTEGER CONSTRAINT [intdefault] DEFAULT 1,
            neg_int INTEGER CONSTRAINT [intnegdefault] DEFAULT -1,
            float REAL CONSTRAINT [float_def] DEFAULT 2.1,
            neg_float REAL CONSTRAINT [negfloat_def] DEFAULT -2.1,
            big_int BIGINT CONSTRAINT [bigint_def] DEFAULT 3,
            neg_big_int BIGINT CONSTRAINT [neg_bigint_def] DEFAULT -3,

            CONSTRAINT [Blog_pkey] PRIMARY KEY (id)
        )
        "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model Blog {
          id          Int     @id @default(autoincrement())
          int         Int?    @default(1, map: "intdefault")
          neg_int     Int?    @default(-1, map: "intnegdefault")
          float       Float?  @default(2.1, map: "float_def") @db.Real
          neg_float   Float?  @default(-2.1, map: "negfloat_def") @db.Real
          big_int     BigInt? @default(3, map: "bigint_def")
          neg_big_int BigInt? @default(-3, map: "neg_bigint_def")
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_table_with_descending_primary_key(api: &mut TestApi) -> TestResult {
    let setup = formatdoc! {r#"
       CREATE TABLE [{}].[A] (
           id INTEGER IDENTITY,
           CONSTRAINT [A_pkey] PRIMARY KEY (id DESC)
       ) 
   "#, api.schema_name()};

    api.raw_cmd(&setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id(sort: Desc) @default(autoincrement())
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_table_with_descending_unique(api: &mut TestApi) -> TestResult {
    let setup = formatdoc! {r#"
       CREATE TABLE [{}].[A] (
           id INTEGER IDENTITY,
           a  INTEGER NOT NULL,
           CONSTRAINT [A_pkey] PRIMARY KEY (id),
           CONSTRAINT [A_a_key] UNIQUE (a DESC)
       ) 
   "#, api.schema_name()};

    api.raw_cmd(&setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          a  Int @unique(sort: Desc)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_table_with_descending_compound_unique(api: &mut TestApi) -> TestResult {
    let setup = formatdoc! {r#"
       CREATE TABLE [{}].[A] (
           id INTEGER IDENTITY,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT [A_pkey] PRIMARY KEY (id),
           CONSTRAINT [A_a_b_key] UNIQUE (a ASC, b DESC)
       )
   "#, api.schema_name()};

    api.raw_cmd(&setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_table_with_descending_index(api: &mut TestApi) -> TestResult {
    let setup = formatdoc! {r#"
       CREATE TABLE [{schema_name}].[A] (
           id INTEGER IDENTITY,
           a  INTEGER NOT NULL,
           b  INTEGER NOT NULL,
           CONSTRAINT [A_pkey] PRIMARY KEY (id)
       );

       CREATE INDEX A_a_b_idx ON [{schema_name}].[A] (a ASC, b DESC);
   "#, schema_name = api.schema_name()};

    api.raw_cmd(&setup).await;

    let expectation = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
