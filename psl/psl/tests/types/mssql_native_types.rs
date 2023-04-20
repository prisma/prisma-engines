use crate::common::*;
use psl::builtin_connectors::{MsSqlType, MsSqlTypeParameter::*};

#[test]
fn text_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Text
          lastName  String @db.Text

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Text` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn ntext_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.NText
          lastName  String @db.NText

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `NText` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varchar_max_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.VarChar(Max)
          lastName  String @db.VarChar(Max)

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `VarChar(Max)` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn nvarchar_max_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.NVarChar(Max)
          lastName  String @db.NVarChar(Max)

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `NVarChar(Max)` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn xml_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
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
        [1;91merror[0m: [1mNative type `Xml` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varbinary_max_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.VarBinary(Max)
          lastName  Bytes @db.VarBinary(Max)

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `VarBinary(Max)` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn image_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Image
          lastName  Bytes @db.Image

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Image` cannot be unique in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn text_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Text
          lastName  String @db.Text

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Text` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn ntext_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.NText
          lastName  String @db.NText

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `NText` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varchar_max_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.VarChar(Max)
          lastName  String @db.VarChar(Max)

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `VarChar(Max)` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn nvarchar_max_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.NVarChar(Max)
          lastName  String @db.NVarChar(Max)

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `NVarChar(Max)` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn xml_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
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
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Xml` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varbinary_max_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.VarBinary(Max)
          lastName  Bytes @db.VarBinary(Max)

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `VarBinary(Max)` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn image_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Image
          lastName  Bytes @db.Image

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Image` of SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn text_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.Text
          lastName  String @db.Text

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Text` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn ntext_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.NText
          lastName  String @db.NText

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `NText` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varchar_max_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.VarChar(Max)
          lastName  String @db.VarChar(Max)

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `VarChar(Max)` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn nvarchar_max_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.NVarChar(Max)
          lastName  String @db.NVarChar(Max)

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `NVarChar(Max)` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn xml_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.Xml
          lastName  String @db.Xml

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Xml` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varbinary_max_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.VarBinary(Max)
          lastName  Bytes @db.VarBinary(Max)

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `VarBinary(Max)` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn image_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.Image
          lastName  Bytes @db.Image

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Image` of SQL Server cannot be used on a field that is `@id` or `@@id`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int     @id
          dec Decimal @db.Decimal(2,4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int     @id
        [1;94m 8 | [0m  dec Decimal [1;91m@db.Decimal(2,4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.Char(8001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Char(8001)` of SQL Server: Length can range from 1 to 8000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int    @id
        [1;94m 8 | [0m  dec String [1;91m@db.Char(8001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nchar_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.NChar(4001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `NChar(4001)` of SQL Server: Length can range from 1 to 4000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int    @id
        [1;94m 8 | [0m  dec String [1;91m@db.NChar(4001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.VarChar(8001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `VarChar(8001)` of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int    @id
        [1;94m 8 | [0m  dec String [1;91m@db.VarChar(8001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nvarchar_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int    @id
          dec String @db.NVarChar(4001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `NVarChar(4001)` of SQL Server: Length can range from 1 to 4000. For larger sizes, use the `Max` variant.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int    @id
        [1;94m 8 | [0m  dec String [1;91m@db.NVarChar(4001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varbinary_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int   @id
          dec Bytes @db.VarBinary(8001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `VarBinary(8001)` of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  dec Bytes [1;91m@db.VarBinary(8001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_binary_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id  Int   @id
          dec Bytes @db.Binary(8001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Binary(8001)` of SQL Server: Length can range from 1 to 8000.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int   @id
        [1;94m 8 | [0m  dec Bytes [1;91m@db.Binary(8001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int      @id
          bigInt DateTime @db.Bit
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Bit is not compatible with declared field type DateTime, expected field type Boolean or Int.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int      @id
        [1;94m 8 | [0m  bigInt DateTime [1;91m@db.Bit[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_bad_type_params() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int    @id
          s      String @db.NVarChar(Ma)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected an integer or `Max`, but found (Ma).[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  s      String [1;91m@db.NVarChar(Ma)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_too_many_type_params() {
    let schema = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int    @id
          s      String @db.NVarChar(1, 2)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type NVarChar takes 1 optional arguments, but received 2.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  s      String [1;91m@db.NVarChar(1, 2)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

macro_rules! test_type {
    ($name:ident($(($input:expr, $output:expr)),+ $(,)?)) => {
            #[test]
            fn $name () {
                $(
                    let input = $input;

                    let dml = formatdoc!(r#"
                        datasource db {{
                          provider = "sqlserver"
                          url      = env("DATABASE_URL")
                        }}

                        model Blog {{
                            id Int    @id
                            x  {input}
                        }}
                    "#);

                    let schema = psl::parse_schema(&dml).unwrap();

                    schema
                        .assert_has_model("Blog")
                        .assert_has_scalar_field("x")
                        .assert_native_type(schema.connector, &$output);
                )+
            }
        }
}

mod test_type_mapping {
    use super::*;

    test_type!(tinyint(("Int @db.TinyInt", MsSqlType::TinyInt)));
    test_type!(smallint(("Int @db.SmallInt", MsSqlType::SmallInt)));
    test_type!(int(("Int @db.Int", MsSqlType::Int)));
    test_type!(money(("Float @db.Money", MsSqlType::Money)));
    test_type!(smallmoney(("Float @db.SmallMoney", MsSqlType::SmallMoney)));
    test_type!(real(("Float @db.Real", MsSqlType::Real)));
    test_type!(date(("DateTime @db.Date", MsSqlType::Date)));
    test_type!(time(("DateTime @db.Time", MsSqlType::Time)));
    test_type!(datetime(("DateTime @db.DateTime", MsSqlType::DateTime)));
    test_type!(datetime2(("DateTime @db.DateTime2", MsSqlType::DateTime2)));
    test_type!(text(("String @db.Text", MsSqlType::Text)));
    test_type!(ntext(("String @db.NText", MsSqlType::NText)));
    test_type!(image(("Bytes @db.Image", MsSqlType::Image)));
    test_type!(xml(("String @db.Xml", MsSqlType::Xml)));

    test_type!(datetimeoffset((
        "DateTime @db.DateTimeOffset",
        MsSqlType::DateTimeOffset
    )));

    test_type!(smalldatetime(("DateTime @db.SmallDateTime", MsSqlType::SmallDateTime)));

    test_type!(binary(
        ("Bytes @db.Binary", MsSqlType::Binary(None)),
        ("Bytes @db.Binary(4000)", MsSqlType::Binary(Some(4000)))
    ));

    test_type!(varbinary(
        ("Bytes @db.VarBinary", MsSqlType::VarBinary(None)),
        ("Bytes @db.VarBinary(4000)", MsSqlType::VarBinary(Some(Number(4000)))),
        ("Bytes @db.VarBinary(Max)", MsSqlType::VarBinary(Some(Max))),
    ));

    test_type!(char(
        ("String @db.Char", MsSqlType::Char(None)),
        ("String @db.Char(4000)", MsSqlType::Char(Some(4000)))
    ));

    test_type!(nchar(
        ("String @db.NChar", MsSqlType::NChar(None)),
        ("String @db.NChar(4000)", MsSqlType::NChar(Some(4000)))
    ));

    test_type!(varchar(
        ("String @db.VarChar", MsSqlType::VarChar(None)),
        ("String @db.VarChar(8000)", MsSqlType::VarChar(Some(Number(8000)))),
        ("String @db.VarChar(Max)", MsSqlType::VarChar(Some(Max))),
    ));

    test_type!(nvarchar(
        ("String @db.NVarChar", MsSqlType::NVarChar(None)),
        ("String @db.NVarChar(4000)", MsSqlType::NVarChar(Some(Number(4000)))),
        ("String @db.NVarChar(Max)", MsSqlType::NVarChar(Some(Max))),
    ));

    test_type!(boolean(
        ("Boolean @db.Bit", MsSqlType::Bit),
        ("Int @db.Bit", MsSqlType::Bit),
    ));

    test_type!(decimal(
        ("Decimal @db.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @db.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(number(
        ("Decimal @db.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @db.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(float(
        ("Float @db.Float", MsSqlType::Float(None)),
        ("Float @db.Float(24)", MsSqlType::Float(Some(24))),
        ("Float @db.Float(53)", MsSqlType::Float(Some(53))),
    ));
}
