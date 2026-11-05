use crate::common::*;
use psl::{
    builtin_connectors::{KINGBASE_MYSQL, KingbaseMySqlType},
    datamodel_connector::NativeTypeInstance,
    parser_database::ScalarType,
};

#[test]
fn kingbase_mysql_rejects_index_length_prefixes() {
    for attribute in [
        "@@index([value(length: 8)])",
        "@@unique([value(length: 8)])",
        "@@id([id, value(length: 8)])",
        "@unique(length: 8)",
        "@id(length: 8)",
    ] {
        let field_attribute = if attribute.starts_with("@@") { "" } else { attribute };
        let model_attribute = if attribute.starts_with("@@") { attribute } else { "" };
        let id_attribute = if attribute.contains("@id") { "" } else { "@id" };
        let schema = format!(
            "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n \
             model Test {{\n id Int {id_attribute}\n value String @db.VarChar(32) {field_attribute}\n \
             {model_attribute}\n }}"
        );
        let error = parse_unwrap_err(&schema);
        assert!(
            error.contains("length argument is not supported"),
            "{attribute}: {error}"
        );
    }
}

#[test]
fn constraint_names_are_limited_to_sixty_three_bytes() {
    let valid_name = "a".repeat(63);
    let invalid_name = "a".repeat(64);

    assert_valid(&format!(
        r#"
        datasource db {{
          provider = "kingbase-mysql"
        }}

        model User {{
          id    Int    @id
          email String

          @@unique([email], map: "{valid_name}")
        }}
        "#
    ));

    let error = parse_unwrap_err(&format!(
        r#"
        datasource db {{
          provider = "kingbase-mysql"
        }}

        model User {{
          id    Int    @id
          email String

          @@unique([email], map: "{invalid_name}")
        }}
        "#
    ));

    assert!(error.contains("The maximum allowed length is 63 bytes."));
}

#[test]
fn text_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.Text
          lastName  String @db.Text

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longtext_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.LongText
          lastName  String @db.LongText

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumtext_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinytext_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn blob_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longblob_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumblob_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinyblob_type_can_be_unique() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@unique([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn text_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.Text
          lastName  String @db.Text

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longtext_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.LongText
          lastName  String @db.LongText

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumtext_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinytext_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn blob_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longblob_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumblob_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinyblob_type_can_be_indexed() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@index([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn text_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName String @db.Text
          lastName  String @db.Text

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longtext_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName String @db.LongText
          lastName  String @db.LongText

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumtext_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName String @db.MediumText
          lastName  String @db.MediumText

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinytext_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName String @db.TinyText
          lastName  String @db.TinyText

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn blob_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName Bytes @db.Blob
          lastName  Bytes @db.Blob

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn longblob_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName Bytes @db.LongBlob
          lastName  Bytes @db.LongBlob

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn mediumblob_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName Bytes @db.MediumBlob
          lastName  Bytes @db.MediumBlob

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn tinyblob_type_can_be_a_primary_key() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          firstName Bytes @db.TinyBlob
          lastName  Bytes @db.TinyBlob

          @@id([firstName, lastName])
        }
    "#};

    assert_valid(schema);
}

#[test]
fn text_should_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id Int    @id
          a  String @db.Text

          @@index([a(length: 30)])
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn text_should_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id Int    @id
          a  String @db.Text @unique(length: 30)
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn text_should_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id String @id(length: 30) @db.Text
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn bytes_should_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id Int   @id
          a  Bytes @db.Blob

          @@index([a(length: 30)])
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn bytes_should_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id Int   @id
          a  Bytes @db.Blob @unique(length: 30)
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn bytes_should_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model A {
          id Bytes @id(length: 30) @db.Blob
        }
    "#};

    assert!(parse_unwrap_err(dml).contains("length argument is not supported"))
}

#[test]
fn should_fail_on_argument_for_bit_0_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Bit(0)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(0)` of Kingbase MySQL: M can range from 1 to 64.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int   @id
        [1;94m 7 | [0m  firstName Bytes [1;91m@db.Bit(0)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_for_bit_65_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int   @id
          firstName Bytes @db.Bit(65)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(65)` of Kingbase MySQL: M can range from 1 to 64.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int   @id
        [1;94m 7 | [0m  firstName Bytes [1;91m@db.Bit(65)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_only_allow_bit_one_for_booleans() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int     @id
          firstName Boolean @db.Bit(2)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Bit(2)` of Kingbase MySQL: only Bit(1) can be used as Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int     @id
        [1;94m 7 | [0m  firstName Boolean [1;91m@db.Bit(2)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.Char(10485761)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Char(10485761)` of Kingbase MySQL: M can range from 0 to 10,485,760.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int    @id
        [1;94m 7 | [0m  firstName String [1;91m@db.Char(10485761)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int    @id
          firstName String @db.VarChar(10485761)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `VarChar(10485761)` of Kingbase MySQL: M can range from 0 to 10,485,760.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int    @id
        [1;94m 7 | [0m  firstName String [1;91m@db.VarChar(10485761)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn should_fail_on_argument_out_of_range_for_decimal_type() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(1001,20)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(1001,20)` of Kingbase MySQL: Precision can range from 1 to 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int     @id
        [1;94m 7 | [0m  firstName Decimal [1;91m@db.Decimal(1001,20)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);

    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int     @id
          firstName Decimal @db.Decimal(44,1001)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Decimal(44,1001)` of Kingbase MySQL: Scale can range from 0 to 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int     @id
        [1;94m 7 | [0m  firstName Decimal [1;91m@db.Decimal(44,1001)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn decimal_scale_larger_than_precision_is_normalized() {
    let mut diagnostics = psl::diagnostics::Diagnostics::new();
    let native_type = KINGBASE_MYSQL
        .parse_native_type(
            "Decimal",
            &["2".into(), "4".into()],
            psl::diagnostics::Span::empty(),
            &mut diagnostics,
        )
        .unwrap();
    assert_eq!(
        native_type.downcast_ref::<KingbaseMySqlType>(),
        &KingbaseMySqlType::Decimal(Some((4, 4)))
    );
    assert_valid(
        "datasource db {\n provider = \"kingbase-mysql\"\n }\n model Test {\n id Int @id\n value Decimal @db.Decimal(2, 4)\n }",
    );
}

#[test]
fn kingbase_native_type_ranges_are_accepted() {
    for native_type in [
        "Char(0)",
        "Char(256)",
        "Char(655350)",
        "Char(10485760)",
        "VarChar(0)",
        "VarChar(655350)",
        "VarChar(10485760)",
        "Decimal(66,20)",
        "Decimal(44,33)",
        "Decimal(1000,1000)",
    ] {
        let scalar = if native_type.starts_with("Decimal") {
            "Decimal"
        } else {
            "String"
        };
        assert_valid(&format!(
            "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n model Test {{\n id Int @id\n value {scalar} @db.{native_type}\n }}"
        ));
    }
}

#[test]
fn zero_decimal_precision_is_rejected_even_when_scale_is_larger() {
    for scale in [0, 4] {
        let schema = format!(
            "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n model Test {{\n id Int @id\n value Decimal @db.Decimal(0, {scale})\n }}"
        );
        assert!(parse_unwrap_err(&schema).contains("Precision can range from 1 to 1000."));
    }
}

#[test]
fn explicit_default_bytes_type_has_the_same_key_validity() {
    for annotation in ["", "@db.LongBlob"] {
        for key in ["@id", "@unique", "@@index([value])"] {
            let (field_key, model_key, id) = if key.starts_with("@@") {
                ("", key, "id Int @id")
            } else if key == "@unique" {
                (key, "", "id Int @id")
            } else {
                (key, "", "")
            };
            assert_valid(&format!(
                "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n model Test {{\n {id}\n value Bytes {field_key} {annotation}\n {model_key}\n }}"
            ));
        }
    }
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model Blog {
          id     Int      @id
          bigInt DateTime @db.TinyInt
        }
    "#;

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type TinyInt is not compatible with declared field type DateTime, expected field type Boolean or Int.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m          id     Int      @id
        [1;94m 8 | [0m          bigInt DateTime [1;91m@db.TinyInt[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn kingbase_mysql_provider_accepts_mysql_compatible_native_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id        Int      @id @db.Int
          email     String   @unique @db.VarChar(191)
          active    Boolean  @db.TinyInt
          score     Decimal  @db.Decimal(10, 2)
          createdAt DateTime @db.DateTime(3)
          payload   Bytes    @db.LongBlob
          metadata  Json     @db.Json
        }
    "#};

    assert_valid(schema);
}

#[test]
fn kingbase_mysql_uses_independent_native_type_defaults() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model User {
          id Int @id
        }
    "#};
    let validated_schema = parse_schema(schema);
    use psl::builtin_connectors::KingbaseMySqlType::*;
    let expected = [
        (ScalarType::Int, Int),
        (ScalarType::BigInt, BigInt),
        (ScalarType::Float, Double),
        (ScalarType::Decimal, Decimal(Some((65, 30)))),
        (ScalarType::Boolean, TinyInt),
        (ScalarType::String, VarChar(191)),
        (ScalarType::DateTime, DateTime(Some(3))),
        (ScalarType::Bytes, LongBlob),
        (ScalarType::Json, Json),
    ];
    for (scalar_type, expected_native_type) in expected {
        let native_type = KINGBASE_MYSQL
            .default_native_type_for_scalar_type(
                &psl::parser_database::ScalarFieldType::BuiltInScalar(scalar_type),
                &validated_schema,
            )
            .unwrap();
        assert_eq!(
            native_type,
            NativeTypeInstance::new::<KingbaseMySqlType>(expected_native_type)
        );
    }
}

#[test]
fn small_unsigned_native_types_are_rejected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model Counter {
          id Int @id
          tiny Int @db.UnsignedTinyInt
          small Int @db.UnsignedSmallInt
          medium Int @db.UnsignedMediumInt
        }
    "#};
    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Small unsigned integer native types are not supported by this Kingbase MySQL connector. Use a supported native type or migrate the column explicitly; Prisma does not emulate these types with domains or triggers.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id Int @id
        [1;94m 7 | [0m  tiny Int [1;91m@db.UnsignedTinyInt[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Small unsigned integer native types are not supported by this Kingbase MySQL connector. Use a supported native type or migrate the column explicitly; Prisma does not emulate these types with domains or triggers.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  tiny Int @db.UnsignedTinyInt
        [1;94m 8 | [0m  small Int [1;91m@db.UnsignedSmallInt[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Small unsigned integer native types are not supported by this Kingbase MySQL connector. Use a supported native type or migrate the column explicitly; Prisma does not emulate these types with domains or triggers.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  small Int @db.UnsignedSmallInt
        [1;94m 9 | [0m  medium Int [1;91m@db.UnsignedMediumInt[0m
        [1;94m   | [0m
    "#]];
    expect_error(schema, &expected);
}

#[test]
fn server_native_unsigned_types_and_boolean_remain_supported() {
    assert_valid(indoc! {r#"
        datasource db {
          provider = "kingbase-mysql"
        }

        model Counter {
          id Int @id
          small Int @db.SmallInt
          tiny Int @db.TinyInt
          medium Int @db.MediumInt
          unsigned Int @db.UnsignedInt
          bigUnsigned BigInt @db.UnsignedBigInt
          enabled Boolean
        }
    "#});
}
