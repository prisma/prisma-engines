use crate::common::*;
use psl::{
    builtin_connectors::{KINGBASE_ORACLE, KingbaseOracleNumberArguments, KingbaseOracleType},
    datamodel_connector::{ConnectorCapability, NativeTypeInstance},
    parser_database::{ReferentialAction, ScalarFieldType, ScalarType},
};

use KingbaseOracleNumberArguments::*;
use KingbaseOracleType::*;

#[test]
fn provider_and_core_native_types_are_valid() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id        Int      @id @db.Number(10, 0)
          tiny      Int      @db.TinyInt
          big       BigInt   @db.Number(19)
          ratio     Float    @db.BinaryDouble
          amount    Decimal  @db.Number(65, 30)
          digit     Int      @db.Number(1, 0)
          active    Boolean  @db.Boolean
          name      String   @db.VarChar2(191)
          createdAt DateTime @db.Timestamp(3)
          payload   Bytes    @db.Blob
          metadata  Json     @db.Json
        }
    "#};

    assert_valid(schema);
}

#[test]
fn default_scalar_types_use_oracle_compatible_native_types() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id Int @id
        }
    "#};
    let validated_schema = parse_schema(schema);
    let expected = [
        (ScalarType::Int, Number(PrecisionAndScale(10, 0))),
        (ScalarType::BigInt, Number(PrecisionAndScale(19, 0))),
        (ScalarType::Float, BinaryDouble),
        (ScalarType::Decimal, Number(PrecisionAndScale(65, 30))),
        (ScalarType::Boolean, Boolean),
        (ScalarType::String, VarChar2(Some(4000))),
        (ScalarType::DateTime, Timestamp(Some(3))),
        (ScalarType::Bytes, Blob),
        (ScalarType::Json, Json),
    ];

    for (scalar_type, expected_native_type) in expected {
        let native_type = KINGBASE_ORACLE
            .default_native_type_for_scalar_type(&ScalarFieldType::BuiltInScalar(scalar_type), &validated_schema)
            .unwrap();

        assert_eq!(
            native_type,
            NativeTypeInstance::new::<KingbaseOracleType>(expected_native_type),
        );
    }
}

#[test]
fn number_accepts_zero_one_or_two_arguments() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model NumberTypes {
          id      Int     @id
          any     Decimal @db.Number
          integer BigInt  @db.Number(19)
          decimal Decimal @db.Number(30, 10)
        }
    "#};

    assert_valid(schema);
}

#[test]
fn number_cannot_be_used_for_boolean_fields() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id     Int     @id
          active Boolean @db.Number(1)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Number is not compatible with declared field type Boolean, expected field type Int or BigInt or Float or Decimal.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id     Int     @id
        [1;94m 7 | [0m  active Boolean [1;91m@db.Number(1)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn invalid_number_precision_is_rejected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id    Int     @id
          value Decimal @db.Number(1001, 1)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Number(1001,1)` of Kingbase Oracle: Precision must be between 1 and 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id    Int     @id
        [1;94m 7 | [0m  value Decimal [1;91m@db.Number(1001, 1)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn number_scale_above_precision_is_normalized() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id    Int     @id
          value Decimal @db.Number(2, 3)
        }
    "#};

    assert_valid(schema);
    let mut diagnostics = psl::diagnostics::Diagnostics::new();
    let parsed = KINGBASE_ORACLE
        .parse_native_type(
            "Number",
            &["2".into(), "3".into()],
            psl::diagnostics::Span::empty(),
            &mut diagnostics,
        )
        .unwrap();
    assert_eq!(
        parsed.downcast_ref::<KingbaseOracleType>(),
        &Number(PrecisionAndScale(3, 3))
    );
}

#[test]
fn normalizing_number_does_not_hide_zero_precision() {
    let schema = r#"datasource db {
          provider = "kingbase-oracle"
        }
        model Item {
          id Int @id
          value Decimal @db.Number(0,4)
        }"#;
    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Number(0,4)` of Kingbase Oracle: Precision must be between 1 and 1000.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m          id Int @id
        [1;94m 6 | [0m          value Decimal [1;91m@db.Number(0,4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn negative_number_scale_reports_the_server_semantics_limitation() {
    let schema = r#"datasource db {
          provider = "kingbase-oracle"
        }
        model Item {
          id Int @id
          value Decimal @db.Number(6,-2)
        }"#;
    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Number` of Kingbase Oracle: Negative NUMBER scale is not supported: this Kingbase Oracle version does not preserve Oracle rounding semantics.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m          id Int @id
        [1;94m 6 | [0m          value Decimal [1;91m@db.Number(6,-2)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn float_precision_above_53_is_rejected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id    Int   @id
          value Float @db.Float(54)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Float(54)` of Kingbase Oracle: Precision must be between 1 and 53.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id    Int   @id
        [1;94m 7 | [0m  value Float [1;91m@db.Float(54)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn varchar2_length_above_server_limit_is_rejected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id    Int    @id
          value String @db.VarChar2(10485761)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `VarChar2(10485761)` of Kingbase Oracle: Length must be between 1 and 10,485,760.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id    Int    @id
        [1;94m 7 | [0m  value String [1;91m@db.VarChar2(10485761)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn timestamp_precision_above_six_is_rejected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id        Int      @id
          localTime DateTime @db.Timestamp(7)
          zonedTime DateTime @db.TimestampTz(7)
          dbTime    DateTime @db.TimestampLocalTz(7)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Timestamp(7)` of Kingbase Oracle: Precision must be between 0 and 6.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id        Int      @id
        [1;94m 7 | [0m  localTime DateTime [1;91m@db.Timestamp(7)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `TimestampTz(7)` of Kingbase Oracle: Precision must be between 0 and 6.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  localTime DateTime @db.Timestamp(7)
        [1;94m 8 | [0m  zonedTime DateTime [1;91m@db.TimestampTz(7)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mArgument M is out of range for native type `TimestampLocalTz(7)` of Kingbase Oracle: Precision must be between 0 and 6.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  zonedTime DateTime @db.TimestampTz(7)
        [1;94m 9 | [0m  dbTime    DateTime [1;91m@db.TimestampLocalTz(7)[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}

#[test]
fn supported_lob_types_can_be_used_as_unique_keys() {
    let schema = indoc! {r#"
        datasource db {
          provider = "kingbase-oracle"
        }

        model User {
          id           Int    @id
          blobValue    Bytes  @unique @db.Blob
          clobValue    String @unique @db.Clob
          nationalClob String @unique @db.NClob
        }
    "#};

    assert_valid(schema);
}

#[test]
fn json_types_cannot_be_used_as_keys_with_or_without_an_explicit_native_type() {
    let cases = [
        (
            indoc! {r#"
            datasource db {
              provider = "kingbase-oracle"
            }

            model User {
              id       Int  @id
              metadata Json @unique
            }
        "#},
            expect![[r#"
                [1;91merror[0m: [1mNative type `Json` cannot be unique in Kingbase Oracle.[0m
                  [1;94m-->[0m  [4mschema.prisma:7[0m
                [1;94m   | [0m
                [1;94m 6 | [0m  id       Int  @id
                [1;94m 7 | [0m  metadata Json [1;91m@unique[0m
                [1;94m   | [0m
            "#]],
        ),
        (
            indoc! {r#"
            datasource db {
              provider = "kingbase-oracle"
            }

            model User {
              id       Int  @id
              metadata Json @unique @db.Json
            }
        "#},
            expect![[r#"
                [1;91merror[0m: [1mNative type `Json` cannot be unique in Kingbase Oracle.[0m
                  [1;94m-->[0m  [4mschema.prisma:7[0m
                [1;94m   | [0m
                [1;94m 6 | [0m  id       Int  @id
                [1;94m 7 | [0m  metadata Json [1;91m@unique [0m@db.Json
                [1;94m   | [0m
            "#]],
        ),
    ];

    for (schema, expectation) in cases {
        expect_error(schema, &expectation);
    }
}

#[test]
fn connector_only_advertises_verified_query_and_referential_action_capabilities() {
    let capabilities = KINGBASE_ORACLE.capabilities();

    assert!(capabilities.contains(ConnectorCapability::InsertReturning));
    assert!(capabilities.contains(ConnectorCapability::UpdateReturning));
    assert!(capabilities.contains(ConnectorCapability::DeleteReturning));
    assert!(capabilities.contains(ConnectorCapability::JsonFiltering));
    assert!(capabilities.contains(ConnectorCapability::JsonFilteringArrayPath));
    assert!(capabilities.contains(ConnectorCapability::JsonFilteringAlphanumeric));
    assert!(capabilities.contains(ConnectorCapability::JsonFilteringAlphanumericFieldRef));
    assert!(capabilities.contains(ConnectorCapability::JsonArrayContains));
    assert!(capabilities.contains(ConnectorCapability::InsensitiveFilters));
    assert!(capabilities.contains(ConnectorCapability::LateralJoin));
    assert!(capabilities.contains(ConnectorCapability::NativeFullTextSearch));
    assert!(capabilities.contains(ConnectorCapability::NativeFullTextSearchWithoutIndex));
    assert!(!capabilities.contains(ConnectorCapability::FullTextIndex));
    assert!(!capabilities.contains(ConnectorCapability::ScalarLists));
    assert!(!capabilities.contains(ConnectorCapability::CorrelatedSubqueries));

    let referential_actions = KINGBASE_ORACLE.foreign_key_referential_actions();
    assert!(referential_actions.contains(ReferentialAction::Restrict));
    assert!(referential_actions.contains(ReferentialAction::SetDefault));
}

#[test]
fn raw_types_are_not_exposed_without_extension_support() {
    assert!(KINGBASE_ORACLE.find_native_type_constructor("Raw").is_none());
    assert!(KINGBASE_ORACLE.find_native_type_constructor("LongRaw").is_none());
}

#[test]
fn connector_requires_the_mode_specific_url_scheme() {
    assert!(KINGBASE_ORACLE.validate_url("kingbase-oracle://localhost/test").is_ok());
    assert!(KINGBASE_ORACLE.validate_url("kingbase://localhost/test").is_err());
    assert!(KINGBASE_ORACLE.validate_url("oracle://localhost/test").is_err());
}

#[test]
fn varchar2_server_supported_lengths_are_valid() {
    for length in [4001, 32767, 10485760] {
        assert_valid(&format!(
            "datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Wide {{\n id Int @id\n v String @db.VarChar2({length})\n}}"
        ));
    }
}
