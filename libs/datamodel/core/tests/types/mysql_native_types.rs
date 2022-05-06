use crate::types::helper::{
    test_native_types_compatibility, test_native_types_with_field_attribute_support,
    test_native_types_without_attributes,
};
use crate::{common::*, with_header, Provider};
use datamodel::parse_schema;
use indoc::indoc;

const BLOB_TYPES: &[&str] = &["Blob", "LongBlob", "MediumBlob", "TinyBlob"];
const TEXT_TYPES: &[&str] = &["Text", "LongText", "MediumText", "TinyText"];

#[test]
fn text_and_blob_data_types_should_fail_on_index() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "You cannot define an index on fields with native type `{}` of MySQL. Please use the `length` argument to the field in the index definition to allow this.",
            type_name
        )
    }

    for tpe in BLOB_TYPES {
        test_block_attribute_support(tpe, "Bytes", "index", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_block_attribute_support(tpe, "String", "index", &error_msg(tpe));
    }
}

#[test]
fn text_should_not_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn text_should_not_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text @unique(length: 30)
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn text_should_not_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        model A {
          id String @id(length: 30) @test.Text
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_index() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Bytes @test.Blob

          @@index([a(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_unique() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Bytes @test.Blob @unique(length: 30)
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn bytes_should_not_fail_on_length_prefixed_pk() {
    let dml = indoc! {r#"
        model A {
          id Bytes @id(length: 30) @test.Blob
        }
    "#};

    let dml = with_header(dml, Provider::Mysql, &[]);

    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn text_and_blob_data_types_can_not_be_unique() {
    fn error_msg(type_name: &str) -> String {
        format!("Native type `{}` cannot be unique in MySQL. Please use the `length` argument to the field in the index definition to allow this.", type_name)
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "unique", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "String", "unique", &error_msg(tpe));
    }
}

#[test]
fn text_and_blob_data_types_should_fail_on_id_attribute() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Native type `{}` of MySQL cannot be used on a field that is `@id` or `@@id`. Please use the `length` argument to the field in the index definition to allow this.",
            type_name
        )
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "id", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "id", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "id", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "String", "id", &error_msg(tpe));
    }
}

fn test_block_attribute_support(native_type: &str, scalar_type: &str, attribute_name: &str, error_msg: &str) {
    let id_field = if attribute_name == "id" {
        ""
    } else {
        "id     Int    @id"
    };
    let dml = format!(
        r#"
    model User {{
      {id_field}
      firstname {scalar_type} @db.{native_type}
      lastname  {scalar_type} @db.{native_type}
      @@{attribute_name}([firstname, lastname])
    }}
    "#,
        id_field = id_field,
        native_type = native_type,
        scalar_type = scalar_type,
        attribute_name = attribute_name
    );

    test_native_types_compatibility(&dml, error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_type() {
    for tpe in &["Bit(0)", "Bit(65)"] {
        let error_msg = format!(
            "Argument M is out of range for native type `{}` of MySQL: M can range from 1 to 64.",
            &tpe
        );
        test_native_types_without_attributes(tpe, "Bytes", &error_msg, MYSQL_SOURCE);
    }
}

#[test]
fn should_only_allow_bit_one_for_booleans() {
    let expected_error =
        "Argument M is out of range for native type `Bit(2)` of MySQL: only Bit(1) can be used as Boolean.";

    test_native_types_without_attributes("Bit(2)", "Boolean", expected_error, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let error_msg = "Argument M is out of range for native type `Char(256)` of MySQL: M can range from 0 to 255.";

    test_native_types_without_attributes("Char(256)", "String", error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let error_msg =
        "Argument M is out of range for native type `VarChar(655350)` of MySQL: M can range from 0 to 65,535.";

    test_native_types_without_attributes("VarChar(655350)", "String", error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_decimal_type() {
    fn error_msg(type_name: &str, arg: &str, range: &str) -> String {
        format!(
            "Argument M is out of range for native type `{}` of MySQL: {} can range from {}.",
            type_name, arg, range
        )
    }

    let native_type = "Decimal(66,20)";

    test_native_types_without_attributes(
        native_type,
        "Decimal",
        &error_msg(native_type, "Precision", "1 to 65"),
        MYSQL_SOURCE,
    );

    let native_type = "Decimal(44,33)";

    test_native_types_without_attributes(
        native_type,
        "Decimal",
        &error_msg(native_type, "Scale", "0 to 30"),
        MYSQL_SOURCE,
    );
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc!(
        r#"
        datasource db {
          provider = "mysql"
          url      = "mysql://"
        }

        model Blog {
            id     Int  @id
            dec Decimal @db.Decimal(2, 4)
        }
        "#
    );

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in MySQL.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id     Int  @id
        [1;94m 8 | [0m    dec Decimal @[1;91mdb.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        model Blog {
            id     Int      @id
            bigInt DateTime @db.TinyInt
        }
    "#;

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type TinyInt is not compatible with declared field type DateTime, expected field type Boolean or Int.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m            id     Int      @id
        [1;94m 9 | [0m            bigInt DateTime @[1;91mdb.TinyInt[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
