use crate::common::*;

#[test]
fn text_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
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
        [1;91merror[0m: [1mNative type `Text` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longtext_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.LongText
          lastName  String @db.LongText

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `LongText` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumtext_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `MediumText` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinytext_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `TinyText` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn blob_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Blob` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longblob_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `LongBlob` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumblob_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `MediumBlob` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinyblob_type_should_fail_on_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@unique([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `TinyBlob` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
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
          provider = "mysql"
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
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Text` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longtext_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.LongText
          lastName  String @db.LongText

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `LongText` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumtext_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `MediumText` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinytext_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `TinyText` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn blob_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `Blob` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longblob_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `LongBlob` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumblob_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `MediumBlob` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@index([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinyblob_type_should_fail_on_index() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@index([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mYou cannot define an index on fields with native type `TinyBlob` of MySQL. Please use the `length` argument to the field in the index definition to allow this.[0m
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
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.Text
          lastName  String @db.Text

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Text` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longtext_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.LongText
          lastName  String @db.LongText

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `LongText` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumtext_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `MediumText` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinytext_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `TinyText` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn blob_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `Blob` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn longblob_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `LongBlob` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn mediumblob_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `MediumBlob` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn tinyblob_type_should_fail_on_id() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@id([firstName, lastName])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type `TinyBlob` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@id([firstName, lastName])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn text_should_not_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id Int    @id
          a  String @db.Text

          @@index([a(length: 30)])
        }
    "#};

    assert_valid(dml)
}

#[test]
fn text_should_not_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id Int    @id
          a  String @db.Text @unique(length: 30)
        }
    "#};

    assert_valid(dml)
}

#[test]
fn text_should_not_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id String @id(length: 30) @db.Text
        }
    "#};

    assert_valid(dml)
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id Int   @id
          a  Bytes @db.Blob

          @@index([a(length: 30)])
        }
    "#};

    assert_valid(dml)
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id Int   @id
          a  Bytes @db.Blob @unique(length: 30)
        }
    "#};

    assert_valid(dml)
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id Bytes @id(length: 30) @db.Blob
        }
    "#};

    assert_valid(dml)
}

#[test]
fn should_fail_on_argument_for_bit_0_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Bit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(0)` of MySQL: M can range from 1 to 64.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int   @id
        [1;94m 8 | [0m  firstName Bytes [1;91m@db.Bit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_for_bit_65_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Bit(65)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(65)` of MySQL: M can range from 1 to 64.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int   @id
        [1;94m 8 | [0m  firstName Bytes [1;91m@db.Bit(65)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_only_allow_bit_one_for_booleans() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int     @id
          firstName Boolean @db.Bit(2)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(2)` of MySQL: only Bit(1) can be used as Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  firstName Boolean [1;91m@db.Bit(2)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Char(256)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Char(256)` of MySQL: M can range from 0 to 255.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  firstName String [1;91m@db.Char(256)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int    @id
          firstName String @db.Char(655350)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Char(655350)` of MySQL: M can range from 0 to 255.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int    @id
        [1;94m 8 | [0m  firstName String [1;91m@db.Char(655350)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(66,20)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(66,20)` of MySQL: Precision can range from 1 to 65.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  firstName Decimal [1;91m@db.Decimal(66,20)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(44,33)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(44,33)` of MySQL: Scale can range from 0 to 30.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id        Int     @id
        [1;94m 8 | [0m  firstName Decimal [1;91m@db.Decimal(44,33)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Blog {
            id     Int  @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in MySQL.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id     Int  @id
        [1;94m 8 | [0m    dec Decimal [1;91m@db.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Blog {
          id     Int      @id
          bigInt DateTime @db.TinyInt
        }
    "#;

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type TinyInt is not compatible with declared field type DateTime, expected field type Boolean or Int.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m          id     Int      @id
        [1;94m 9 | [0m          bigInt DateTime [1;91m@db.TinyInt[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}
