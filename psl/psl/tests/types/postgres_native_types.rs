use crate::common::*;
use psl::builtin_connectors::PostgresType;

#[test]
fn xml_data_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Xml
          lastName  String @db.Xml

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Xml` of Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  [1;91mfirstName String @db.Xml[0m
        [1;94m 9 | [0m  lastName  String @db.Xml
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn xml_data_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Xml
          lastName  String @db.Xml

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Xml` cannot be unique in Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  [1;91mfirstName String @db.Xml[0m
        [1;94m 9 | [0m  lastName  String @db.Xml
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int     @id
          val Decimal @db.Decimal(1001,3)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(1001,3)` of Postgres: Precision must be positive with a maximum value of 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int     @id
        [1;94m 8 | [0m  val Decimal [1;91m@db.Decimal(1001,3)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Time(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Time(7)` of Postgres: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Time(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Timestamp(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Timestamp(7)` of Postgres: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Timestamp(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int   @id
          val Bytes @db.Bit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Bit is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  val Bytes [1;91m@db.Bit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int   @id
          val Bytes @db.VarBit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type VarBit is not compatible with declared field type Bytes, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  val Bytes [1;91m@db.VarBit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int   @id
          dec Decimal @db.Decimal(2, 4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in Postgres.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int   @id
        [1;94m 8 | [0m  dec Decimal [1;91m@db.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn xml_should_work_with_string_scalar_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.Xml
        }
    "#};

    let datamodel = psl::parse_schema(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("dec")
        .assert_native_type(datamodel.connector, &PostgresType::Xml);
}
