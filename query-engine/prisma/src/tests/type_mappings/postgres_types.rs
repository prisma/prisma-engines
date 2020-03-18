use super::test_api::*;
use datamodel::ScalarType;
use indoc::indoc;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_macros::test_each_connector;

const CREATE_TYPES_TABLE: &str = indoc! {
    r##"
    CREATE TABLE "prisma-tests"."types" (
        id SERIAL PRIMARY KEY,
        numeric_int2 int2,
        numeric_int4 int4,
        numeric_int8 int8,

        numeric_decimal decimal(8, 4),
        numeric_float4 float4,
        numeric_float8 float8,

        numeric_serial2 serial2,
        numeric_serial4 serial4,
        numeric_serial8 serial8,

        numeric_money money,
        numeric_oid oid,

        string_char char(8),
        string_varchar varchar(20),
        string_text text,

        binary_bytea bytea,
        binary_bits  bit(7),
        binary_bits_varying bit varying(80),
        binary_uuid uuid,

        time_timestamp timestamp,
        time_timestamptz timestamptz,
        time_date date,
        time_time time,
        time_timetz timetz,
        time_interval interval,

        boolean_boolean boolean,

        network_cidr cidr,
        network_inet inet,
        network_mac  macaddr,

        search_tsvector tsvector,
        search_tsquery tsquery,

        json_json json,
        json_jsonb jsonb,

        range_int4range int4range,
        range_int8range int8range,
        range_numrange numrange,
        range_tsrange tsrange,
        range_tstzrange tstzrange,
        range_daterange daterange
    );
    "##
};

#[test_each_connector(tags("postgres"), log = "debug")]
async fn postgres_types_roundtrip(api: &TestApi) -> TestResult {
    api.execute_sql(CREATE_TYPES_TABLE).await?;

    let (datamodel, engine) = api.introspect_and_start_query_engine().await?;

    datamodel.assert_model("types", |model| {
        model
            .assert_field_type("numeric_int2", ScalarType::Int)?
            .assert_field_type("numeric_int4", ScalarType::Int)?
            .assert_field_type("numeric_int8", ScalarType::Int)?
            .assert_field_type("numeric_decimal", ScalarType::Float)?
            .assert_field_type("numeric_float4", ScalarType::Float)?
            .assert_field_type("numeric_float8", ScalarType::Float)?
            .assert_field_type("numeric_serial2", ScalarType::Int)?
            .assert_field_type("numeric_serial4", ScalarType::Int)?
            .assert_field_type("numeric_serial8", ScalarType::Int)?
            .assert_field_type("numeric_money", ScalarType::Float)?
            .assert_field_type("numeric_oid", ScalarType::Int)?
            .assert_field_type("string_char", ScalarType::String)?
            .assert_field_type("string_varchar", ScalarType::String)?
            .assert_field_type("string_text", ScalarType::String)?
            .assert_field_type("binary_bytea", ScalarType::String)?
            .assert_field_type("binary_bits", ScalarType::String)?
            .assert_field_type("binary_bits_varying", ScalarType::String)?
            .assert_field_type("binary_uuid", ScalarType::String)?
            .assert_field_type("time_timestamp", ScalarType::DateTime)?
            .assert_field_type("time_timestamptz", ScalarType::DateTime)?
            .assert_field_type("time_date", ScalarType::DateTime)?
            .assert_field_type("time_time", ScalarType::DateTime)?
            .assert_field_type("time_timetz", ScalarType::DateTime)?
            .assert_field_type("time_interval", ScalarType::String)?
            .assert_field_type("boolean_boolean", ScalarType::Boolean)?
            .assert_field_type("network_cidr", ScalarType::String)?
            .assert_field_type("network_inet", ScalarType::String)?
            .assert_field_type("network_mac", ScalarType::String)?
            .assert_field_type("search_tsvector", ScalarType::String)?
            .assert_field_type("search_tsquery", ScalarType::String)?
            .assert_field_type("json_json", ScalarType::String)?
            .assert_field_type("json_jsonb", ScalarType::String)?
            .assert_field_type("range_int4range", ScalarType::String)?
            .assert_field_type("range_int8range", ScalarType::String)?
            .assert_field_type("range_numrange", ScalarType::String)?
            .assert_field_type("range_tsrange", ScalarType::String)?
            .assert_field_type("range_tstzrange", ScalarType::String)?
            .assert_field_type("range_daterange", ScalarType::String)
    })?;

    let query = indoc! {
        r##"
        mutation {
            createOnetypes(
                data: {
                    numeric_int2: 12
                    numeric_int4: 9002
                    numeric_int8: 100000000
                    numeric_decimal: 49.3444
                    numeric_float4: 12.12
                    numeric_float8: 3.139428
                    numeric_serial2: 8,
                    numeric_serial4: 80,
                    numeric_serial8: 80000,
                    numeric_money: 3.50
                    numeric_oid: 2000
                    string_char: "yeet"
                    string_varchar: "yeet variable"
                    string_text: "to yeet or not to yeet"
                    binary_bits: "0101110"
                    binary_bits_varying: "0101110"
                    # binary_bytea: "test"
                    binary_uuid: "111142ec-880b-4062-913d-8eac479ab957"
                    time_timestamp: "2020-03-02T08:00:00.000"
                    time_timestamptz: "2020-03-02T08:00:00.000"
                    time_date: "2020-03-05T00:00:00.000"
                    time_time: "2020-03-05T08:00:00.000"
                    time_timetz: "2020-03-05T08:00:00.000"
                    # time_interval: "3 hours"
                    boolean_boolean: true
                    # network_cidr: "192.168.100.14/24"
                    network_inet: "192.168.100.14"
                    # network_mac: "12:33:ed:44:49:36"
                    # search_tsvector: "''a'' ''dump'' ''dumps'' ''fox'' ''in'' ''the''"
                    # search_tsquery: "''foxy cat''"
                    json_json: "{ \"isJson\": true }"
                    json_jsonb: "{ \"isJSONB\": true }"
                    # range_int4range: "[-4, 8)"
                    # range_int8range: "[4000, 9000)"
                    # range_numrange: "[11.1, 22.2)"
                    # range_tsrange: "[2010-01-01 14:30, 2010-01-01 15:30)"
                    # range_tstzrange: "[2010-01-01 14:30, 2010-01-01 15:30)"
                    # range_daterange: "[2020-03-02, 2020-03-22)"
                }
            ) {
                numeric_int2
                numeric_int4
                numeric_int8
                numeric_decimal
                numeric_float4
                numeric_float8
                numeric_serial2
                numeric_serial4
                numeric_serial8
                numeric_money
                numeric_oid
                string_char
                string_varchar
                string_text
                # binary_bytea
                binary_bits
                binary_bits_varying
                binary_uuid
                time_timestamp
                time_timestamptz
                time_date
                time_time
                time_timetz
                # time_interval
                boolean_boolean
                # network_cidr
                network_inet
                # network_mac
                # search_tsvector
                # search_tsquery
                json_json
                json_jsonb
                # range_int4range
                # range_int8range
                # range_numrange
                # range_tsrange
                # range_tstzrange
                # range_daterange
            }
        }
        "##
    };

    let response = engine.request(query).await;

    let expected_response = json!({
        "data": {
            "createOnetypes": {
                "numeric_int2": 12,
                "numeric_int4": 9002,
                "numeric_int8": 100000000,
                "numeric_serial2": 8,
                "numeric_serial4": 80,
                "numeric_serial8": 80000,
                "numeric_decimal": 49.3444,
                "numeric_float4": 12.12,
                "numeric_float8": 3.139428,
                "numeric_money": 3.5,
                "numeric_oid": 2000,
                "string_char": "yeet    ",
                "string_varchar": "yeet variable",
                "string_text": "to yeet or not to yeet",
                "binary_bits": "0101110",
                "binary_bits_varying": "0101110",
                "binary_uuid": "111142ec-880b-4062-913d-8eac479ab957",
                "time_timestamp": "2020-03-02T08:00:00.000Z",
                "time_timestamptz": "2020-03-02T08:00:00.000Z",
                "time_date": "2020-03-05T00:00:00.000Z",
                "time_time": "1970-01-01T08:00:00.000Z",
                "time_timetz": "1970-01-01T08:00:00.000Z",
                "boolean_boolean": true,
                "network_inet": "192.168.100.14",
                "json_json": "{\"isJson\":true}",
                "json_jsonb": "{\"isJSONB\":true}",
            }
        }
    });

    assert_eq!(response, expected_response);

    Ok(())
}

#[test_each_connector(tags("postgres"), log = "debug")]
async fn small_float_values_must_work(api: &TestApi) -> TestResult {
    let schema = indoc! {
        r#"
        CREATE TABLE floatilla (
            id SERIAL PRIMARY KEY,
            f32 float4,
            f64 float8,
            decimal_column decimal
        );
        "#
    };

    api.execute_sql(schema).await?;

    let (datamodel, engine) = api.introspect_and_start_query_engine().await?;

    datamodel.assert_model("floatilla", |model| {
        model
            .assert_field_type("f32", ScalarType::Float)?
            .assert_field_type("f64", ScalarType::Float)?
            .assert_field_type("decimal_column", ScalarType::Float)
    })?;

    let query = indoc! {
        r##"
        mutation {
            createOnefloatilla(
                data: {
                    f32: 0.00006927,
                    f64: 0.00006927,
                    decimal_column: 0.00006927
                }
            ) {
                id
                f32
                f64
                decimal_column
            }
        }
        "##
    };

    let response = engine.request(query).await;

    let expected_response = json!({
        "data": {
            "createOnefloatilla": {
                "id": 1,
                "f32": 0.00006927,
                "f64": 0.00006927,
                "decimal_column": 0.00006927
            }
        }
    });

    assert_eq!(response, expected_response);

    Ok(())
}

const CREATE_ARRAY_TYPES_TABLE: &str = indoc! {
    r##"
    CREATE TABLE "prisma-tests"."arraytypes" (
        id SERIAL PRIMARY KEY,
        numeric_int2 int2[],
        numeric_int4 int4[],
        numeric_int8 int8[],

        numeric_decimal decimal(8, 4)[],
        numeric_float4 float4[],
        numeric_float8 float8[],

        numeric_money money[],
        numeric_oid oid[],

        string_char char(8)[],
        string_varchar varchar(20)[],
        string_text text[],

        binary_bytea bytea[],
        binary_bits  bit(8)[],
        binary_bits_varying bit varying(80)[],
        binary_uuid uuid[],

        time_timestamp timestamp[],
        time_timestamptz timestamptz[],
        time_date date[],
        time_time time[],
        time_timetz timetz[],

        boolean_boolean boolean[],

        network_cidr cidr[],
        network_inet inet[],

        json_json json[],
        json_jsonb jsonb[]
    );
    "##
};

#[test_each_connector(tags("postgres"))]
async fn postgres_array_types_roundtrip(api: &TestApi) -> TestResult {
    api.execute_sql(CREATE_ARRAY_TYPES_TABLE).await?;

    let (datamodel, engine) = api.introspect_and_start_query_engine().await?;

    datamodel.assert_model("arraytypes", |model| {
        model
            .assert_field_type("numeric_int2", ScalarType::Int)?
            .assert_field_type("numeric_int4", ScalarType::Int)?
            .assert_field_type("numeric_int8", ScalarType::Int)?
            .assert_field_type("numeric_decimal", ScalarType::Float)?
            .assert_field_type("numeric_float4", ScalarType::Float)?
            .assert_field_type("numeric_float8", ScalarType::Float)?
            .assert_field_type("numeric_money", ScalarType::Float)?
            .assert_field_type("numeric_oid", ScalarType::Int)?
            .assert_field_type("string_char", ScalarType::String)?
            .assert_field_type("string_varchar", ScalarType::String)?
            .assert_field_type("string_text", ScalarType::String)?
            .assert_field_type("binary_bytea", ScalarType::String)?
            .assert_field_type("binary_bits", ScalarType::String)?
            .assert_field_type("binary_bits_varying", ScalarType::String)?
            .assert_field_type("binary_uuid", ScalarType::String)?
            .assert_field_type("time_timestamp", ScalarType::DateTime)?
            .assert_field_type("time_timestamptz", ScalarType::DateTime)?
            .assert_field_type("time_date", ScalarType::DateTime)?
            .assert_field_type("time_time", ScalarType::DateTime)?
            .assert_field_type("time_timetz", ScalarType::DateTime)?
            .assert_field_type("boolean_boolean", ScalarType::Boolean)?
            .assert_field_type("network_inet", ScalarType::String)?
            .assert_field_type("json_json", ScalarType::String)?
            .assert_field_type("json_jsonb", ScalarType::String)
    })?;

    let query = indoc! {
        r##"
        mutation {
            createOnearraytypes(
                data: {
                    numeric_int2: { set: [12] }
                    numeric_int4: { set: [9002] }
                    numeric_int8: { set: [100000000] }
                    numeric_decimal: { set: [49.3444] }
                    numeric_float4: { set: [12.12] }
                    numeric_float8: { set: [3.139428] }
                    numeric_money: { set: [3.50] }
                    numeric_oid: { set: [2000] }
                    string_char: { set: ["yeet"] }
                    string_varchar: { set: ["yeet variable"] }
                    string_text: { set: ["to yeet or not to yeet"] }
                    binary_bits: { set: ["10100011"] }
                    binary_bits_varying: { set: ["01000"] }
                    binary_uuid: { set: ["111142ec-880b-4062-913d-8eac479ab957"] }
                    time_timestamp: { set: ["2020-03-02T08:00:00.000"] }
                    time_timestamptz: { set: ["2020-03-02T08:00:00.000"] }
                    time_date: { set: ["2020-03-05T00:00:00.000"] }
                    time_time: { set: ["2020-03-05T08:00:00.000"] }
                    time_timetz: { set: ["2020-03-05T08:00:00.000"] }
                    boolean_boolean: { set: [true, true, false, true] }
                    network_inet: { set: ["192.168.100.14"] }
                    json_json: { set: ["{ \"isJson\": true }"] }
                    json_jsonb: { set: ["{ \"isJSONB\": true }"] }
                }
            ) {
                numeric_int2
                numeric_int4
                numeric_int8
                numeric_decimal
                numeric_float4
                numeric_float8
                numeric_money
                numeric_oid
                string_char
                string_varchar
                string_text
                binary_bits
                binary_bits_varying
                binary_uuid
                time_timestamp
                time_timestamptz
                time_date
                time_time
                time_timetz
                boolean_boolean
                network_inet
                json_json
                json_jsonb
            }
        }
        "##
    };

    let response = engine.request(query).await;

    let expected_response = json!({
        "data": {
            "createOnearraytypes": {
                "numeric_int2": [12],
                "numeric_int4": [9002],
                "numeric_int8": [100000000],
                "numeric_decimal": [49.3444],
                "numeric_float4": [12.12],
                "numeric_float8": [3.139428],
                "numeric_money": [3.5],
                "numeric_oid": [2000],
                "string_char": ["yeet    "],
                "string_varchar": ["yeet variable"],
                "string_text": ["to yeet or not to yeet"],
                "binary_bits": ["10100011"],
                "binary_bits_varying": ["01000"],
                "binary_uuid": ["111142ec-880b-4062-913d-8eac479ab957"],
                "time_timestamp": ["2020-03-02T08:00:00.000Z"],
                "time_timestamptz": ["2020-03-02T08:00:00.000Z"],
                "time_date": ["2020-03-05T00:00:00.000Z"],
                "time_time": ["1970-01-01T08:00:00.000Z"],
                "time_timetz": ["1970-01-01T08:00:00.000Z"],
                "boolean_boolean": [true, true, false, true],
                "network_inet": ["192.168.100.14"],
                "json_json": ["{\"isJson\":true}"],
                "json_jsonb": ["{\"isJSONB\":true}"],
            }
        }
    });

    assert_eq!(response, expected_response);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn all_postgres_id_types_work(api: &TestApi) -> TestResult {
    let pk_types = &[
        ("int2", "12"),
        ("int4", "78"),
        ("int8", "1293"),
        ("decimal(8, 4)", "2.5"),
        ("float4", "2.8"),
        ("float8", "2.000039"),
        // ("serial2", "12"),
        // ("serial4", "78"),
        // ("serial8", "1293"),
        ("money", "800.2"),
        ("oid", "1288"),
        ("char(8)", "\"manulcat\""),
        ("varchar(20)", "\"the-pk-is-here\""),
        ("text", "\"this is a primary key with spaces\""),
        ("bit(7)", "\"1111000\""),
        ("bit varying(80)", "\"1111000\""),
        ("uuid", "\"111142ec-880b-4062-913d-8eac479ab957\""),
        ("timestamp", "\"2019-01-28T00:03:20.001Z\""),
        ("timestamptz", "\"2019-01-28T00:03:20.001Z\""),
        ("date", "\"2020-01-08T00:00:00.000Z\""),
        ("time", "\"1970-01-01T12:33:00.050Z\""),
        ("timetz", "\"1970-01-01T12:33:00.050Z\""),
        ("boolean", "true"),
        ("inet", "\"127.0.0.4\""),
        // ("json", "\"{ \\\"isThisPrimaryKeyReallyJSON\\\": true }\""),
        // ("jsonb", "\"{ \\\"isThisPrimaryKeyReallyJSON\\\": true }\""),
    ];

    let drop_table = r#"DROP TABLE IF EXISTS "prisma-tests"."pk_test""#;

    for (pk_type, pk_value) in pk_types {
        let create_table = format!(
            r#"CREATE TABLE "prisma-tests"."pk_test" (id {}, PRIMARY KEY (id))"#,
            pk_type
        );
        api.execute_sql(drop_table).await?;
        api.execute_sql(&create_table).await?;

        let (_datamodel, engine) = api.introspect_and_start_query_engine().await?;

        let query = format!(
            r#"
            mutation {{
                createOnepk_test(
                    data: {{
                        id: {}
                    }}
                ) {{
                    id
                }}
            }}
            "#,
            pk_value
        );

        let response = engine.request(query).await;

        assert_eq!(
            response.to_string(),
            format!(r#"{{"data":{{"createOnepk_test":{{"id":{}}}}}}}"#, pk_value)
        );
    }

    Ok(())
}
