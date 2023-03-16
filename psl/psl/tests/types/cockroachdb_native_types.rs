use crate::common::*;
use expect_test::expect;

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(1001,3)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(1001,3)` of CockroachDB: Precision must be positive with a maximum value of 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  firstName Decimal [1;91m@db.Decimal(1001,3)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Time(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Time(7)` of CockroachDB: M can range from 0 to 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int      @id
        [1;94m 8 | [0m  val DateTime [1;91m@db.Time(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model User {
          id  Int      @id
          val DateTime @db.Timestamp(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Timestamp(7)` of CockroachDB: M can range from 0 to 6.[0m
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
          provider = "cockroachdb"
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
          provider = "cockroachdb"
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
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int     @id
          dec Decimal @db.Decimal(2, 4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in CockroachDB.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int     @id
        [1;94m 8 | [0m  dec Decimal [1;91m@db.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn cockroach_specific_native_types_are_valid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "cockroachdb"
          url = env("TEST_DATABASE_URL")
        }

        model NativeTypesTest {
          id          Int      @db.Int4 @id @default(sequence())
          bitcol      String   @db.Bit
          boolcol     Boolean  @db.Bool
          bytescol    Bytes    @db.Bytes
          charcol     String   @db.Char(5)
          datecol     DateTime @db.Date
          decimalcol  Decimal  @db.Decimal(5, 2)
          float4col   Float    @db.Float4
          float8col   Float    @db.Float8
          inetcol     String   @db.Inet
          int2col     Int      @db.Int2
          int4col     Int      @db.Int4
          int8col     BigInt   @db.Int8
          jsonbcol    Json     @db.JsonB
          oidcol      Int      @db.Oid
          scharcol    String   @db.CatalogSingleChar
          stringcol1  String   @db.String
          stringcol2  String   @db.String(40)
          timecol     DateTime @db.Time
          timetzcol   DateTime @db.Timetz
          timestcol   DateTime @db.Timestamp
          timesttzcol DateTime @db.Timestamptz
          uuidcol     String   @db.Uuid
          varbitcol   String   @db.VarBit(200)
        }
    "#};

    psl::parse_schema(schema).unwrap();
}
