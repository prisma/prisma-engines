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
          amount    Decimal  @db.Number(38, 18)
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
        (ScalarType::Decimal, Number(Unspecified)),
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
          value Decimal @db.Number(39, 1)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument M is out of range for native type `Number(39,1)` of Kingbase Oracle: Precision must be between 1 and 38.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id    Int     @id
        [1;94m 7 | [0m  value Decimal [1;91m@db.Number(39, 1)[0m
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
        [1;91merror[0m: [1mArgument M is out of range for native type `Number(0,4)` of Kingbase Oracle: Precision must be between 1 and 38.[0m
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

#[test]
fn number_precision_bounds_do_not_restrict_supported_scale() {
    for native in [
        "Number",
        "Number(1)",
        "Number(38)",
        "Number(38,0)",
        "Number(38,30)",
        "Number(2,50)",
        "Number(38,1000)",
    ] {
        assert_valid(&format!(
            "datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n value Decimal @db.{native}\n}}"
        ));
    }
    for native in [
        "Number(0)",
        "Number(39)",
        "Number(65,30)",
        "Number(39,1000)",
        "Number(1000,1000)",
    ] {
        let error = parse_unwrap_err(&format!(
            "datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n value Decimal @db.{native}\n}}"
        ));
        assert!(
            error.contains("Precision must be between 1 and 38."),
            "{native}: {error}"
        );
    }
    let error = parse_unwrap_err(
        "datasource db {\n provider = \"kingbase-oracle\"\n}\nmodel Item {\n id Int @id\n value Decimal @db.Number(38,1001)\n}",
    );
    assert!(error.contains("Scale must be between 0 and 1000."), "{error}");
}

#[test]
fn oracle_additional_native_types_validate_arguments_and_scalar_mappings() {
    let fields = [
        ("fixed String @db.CharByte(8)", CharByte(8)),
        ("variable String @db.VarChar2Byte(8)", VarChar2Byte(8)),
        ("row String @db.RowId", RowId),
        ("universal String @db.URowId(4000)", URowId(Some(4000))),
        ("unbounded String @db.URowId", URowId(None)),
        ("integer Int @db.BinaryInteger", BinaryInteger),
    ];
    for (field, native) in fields {
        let schema =
            format!("datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n {field}\n}}");
        assert_valid(&schema);
        let instance = NativeTypeInstance::new(native);
        let scalar = KINGBASE_ORACLE
            .scalar_type_for_native_type(&instance, &psl::parser_database::NoExtensionTypes)
            .unwrap();
        assert_eq!(
            scalar,
            ScalarFieldType::BuiltInScalar(if native == BinaryInteger {
                ScalarType::Int
            } else {
                ScalarType::String
            })
        );
    }
    for field in [
        "value String @db.CharByte",
        "value String @db.VarChar2Byte",
        "value String @db.CharByte(0)",
        "value String @db.VarChar2Byte(10485761)",
        "value String @db.URowId(0)",
        "value String @db.URowId(10485761)",
        "value Int @db.RowId",
        "value String @db.BinaryInteger",
    ] {
        let schema =
            format!("datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n {field}\n}}");
        assert!(psl::parse_schema_without_extensions(schema).is_err(), "{field}");
    }
}

#[test]
fn decimal_alias_uses_number_arguments_and_canonical_native_type() {
    let constructor = KINGBASE_ORACLE.find_native_type_constructor("Decimal").unwrap();
    assert_eq!(constructor.number_of_args, 0);
    assert_eq!(constructor.number_of_optional_args, 2);
    for args in [
        vec![],
        vec!["10"],
        vec!["10", "0"],
        vec!["12", "4"],
        vec!["2", "3"],
        vec!["38", "1000"],
    ] {
        let declaration = if args.is_empty() {
            "Decimal".to_owned()
        } else {
            format!("Decimal({})", args.join(","))
        };
        assert_valid(&format!(
            "datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n value Decimal @db.{declaration}\n}}"
        ));
        let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
        let mut diagnostics = psl::diagnostics::Diagnostics::new();
        let alias = KINGBASE_ORACLE
            .parse_native_type("Decimal", &args, psl::diagnostics::Span::empty(), &mut diagnostics)
            .unwrap();
        let number = KINGBASE_ORACLE
            .parse_native_type("Number", &args, psl::diagnostics::Span::empty(), &mut diagnostics)
            .unwrap();
        assert_eq!(alias, number);
        assert_eq!(KINGBASE_ORACLE.native_type_to_parts(&alias).0, "Number");
    }
}

#[test]
fn decimal_alias_reuses_number_validation_and_rejects_incompatible_fields() {
    for (native, expected_error) in [
        ("Decimal(0)", "Precision must be between 1 and 38"),
        ("Decimal(39)", "Precision must be between 1 and 38"),
        ("Decimal(0,2)", "Precision must be between 1 and 38"),
        ("Decimal(39,2)", "Precision must be between 1 and 38"),
        ("Decimal(10,1001)", "Scale must be between 0 and 1000"),
        ("Decimal(10,-1)", "Negative NUMBER scale is not supported"),
        ("Decimal(1,2,3)", "optional arguments"),
    ] {
        let error = parse_unwrap_err(&format!(
            "datasource db {{\n provider = \"kingbase-oracle\"\n}}\nmodel Item {{\n id Int @id\n value Decimal @db.{native}\n}}"
        ));
        assert!(error.contains(expected_error), "{native}: {error}");
    }
    let error = parse_unwrap_err(
        "datasource db {\n provider = \"kingbase-oracle\"\n}\nmodel Item {\n id Int @id\n value String @db.Decimal(10,2)\n}",
    );
    assert!(
        error.contains("not compatible with declared field type String"),
        "{error}"
    );
}
