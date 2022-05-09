use crate::{common::*, types::helper::test_native_types_without_attributes};

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for native type `{}` of CockroachDB: Precision must be positive with a maximum value of 1000.",
            type_name
        )
    }

    let native_type = "Decimal(1001,3)";
    test_native_types_without_attributes(native_type, "Decimal", &error_msg(native_type), COCKROACHDB_SOURCE);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for native type `{}` of CockroachDB: M can range from 0 to 6.",
            type_name
        )
    }

    for tpe in &["Timestamp", "Time"] {
        let native_type = &format!("{}(7)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), COCKROACHDB_SOURCE);
        let native_type = &format!("{}(-1)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), COCKROACHDB_SOURCE);
    }
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for native type `{}` of CockroachDB: M must be a positive integer.",
            type_name
        )
    }

    for tpe in &["Bit", "VarBit"] {
        let native_type = &format!("{}(0)", tpe);
        test_native_types_without_attributes(native_type, "String", &error_msg(native_type), COCKROACHDB_SOURCE);
    }
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "cockroachdb"
          url      = "postgresql://"
        }

        generator js {
            provider        = "prisma-client-js"
            previewFeatures = ["Cockroachdb"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in CockroachDB.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m            id     Int   @id
        [1;94m14 | [0m            dec Decimal @[1;91mdb.Decimal(2, 4)[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expectation);
}

#[test]
fn cockroach_specific_native_types_are_valid() {
    let schema = r#"
    datasource db {
        provider = "cockroachdb"
        url = env("TEST_DATABASE_URL")
    }

    generator js {
        provider = "prisma-client-js"
        previewFeatures = ["cockroachdb"]
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
    "#;

    parse(schema);
}
