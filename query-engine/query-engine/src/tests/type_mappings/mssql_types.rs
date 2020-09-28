use super::test_api::*;
use datamodel::dml::ScalarType;
use indoc::indoc;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

fn create_types_table_sql() -> &'static str {
    r##"
    CREATE TABLE [types] (
        [id] int identity(1,1) primary key,
        [numeric_integer_tinyint] tinyint,
        [numeric_integer_smallint] smallint,
        [numeric_integer_int] int,
        [numeric_integer_bigint] bigint,
        [numeric_floating_decimal] decimal(10,2),
        [numeric_floating_numeric] numeric(10,2),
        [numeric_floating_float] float(24),
        [numeric_floating_double] float(53),
        [numeric_floating_real] real,
        [numeric_floating_money] money,
        [numeric_floating_smallmoney] smallmoney,
        [numeric_boolean] bit,
        [date_date] date,
        [date_time] time,
        [date_datetime] datetime,
        [date_datetime2] datetime2,
        [date_smalldatetime] smalldatetime,
        [date_datetimeoffset] datetimeoffset,
        [text_limited_char] char(10),
        [text_limited_varchar] varchar(255),
        [text_limited_nvarchar] nvarchar(255),
        [text_unlimited_varchar] varchar(max),
        [text_unlimited_nvarchar] nvarchar(max),
        [text_unlimited_text] text,
        [text_unlimited_ntext] ntext
    )
    "##
}

const CREATE_ONE_TYPES_QUERY: &str = indoc! {
    "
    mutation {
        createOnetypes(
            data: {
                numeric_integer_tinyint: 12,
                numeric_integer_smallint: 350,
                numeric_integer_int: 9002,
                numeric_integer_bigint: 30000,
                numeric_floating_decimal: 3.14
                numeric_floating_numeric: 3.14
                numeric_floating_float: -32.0
                numeric_floating_double: 0.14
                numeric_floating_real: 12.12
                numeric_floating_money: 12.12
                numeric_floating_smallmoney: 12.12
                numeric_boolean: true
                date_date: \"2020-02-27T00:00:00Z\"
                date_time: \"1970-01-01T16:20:20Z\"
                date_datetime: \"2020-02-27T19:10:22Z\"
                date_datetime2: \"2020-02-27T19:10:22Z\"
                date_smalldatetime: \"2020-02-27T19:10:22Z\"
                date_datetimeoffset: \"2020-02-27T19:10:22Z\"
                text_limited_char: \"abcdefghij\"
                text_limited_varchar: \"muspus naunau\"
                text_limited_nvarchar: \"余余余余余\"
                text_unlimited_varchar: \"muspus naunau\"
                text_unlimited_nvarchar: \"余余余余余\"
                text_unlimited_text: \"muspus naunau\"
                text_unlimited_ntext: \"余余余余余\"
            }
        ) { id }
    }
    "
};

fn expected_create_one_types_result() -> serde_json::Value {
    json!({
        "numeric_integer_tinyint": 12,
        "numeric_integer_smallint": 350,
        "numeric_integer_int": 9002,
        "numeric_integer_bigint": 30000,
        "numeric_floating_decimal": 3.14,
        "numeric_floating_numeric": 3.14,
        "numeric_floating_float": -32.0,
        "numeric_floating_double": 0.14,
        "numeric_floating_real": 12.12,
        "numeric_floating_money": 12.12,
        "numeric_floating_smallmoney": 12.12,
        "numeric_boolean": true,
        "date_date": "2020-02-27T00:00:00.000Z",
        "date_time": "1970-01-01T16:20:20.000Z",
        "date_datetime": "2020-02-27T19:10:22.000Z",
        "date_datetime2": "2020-02-27T19:10:22.000Z",
        "date_smalldatetime": "2020-02-27T19:10:22.000Z",
        "date_datetimeoffset": "2020-02-27T19:10:22.000Z",
        "date_timestamp": "2020-02-27T19:11:22.000Z",
        "text_limited_char": "abcdefghij",
        "text_limited_varchar": "muspus naunau",
        "text_limited_nvarchar": "余余余余余",
        "text_unlimited_varchar": "muspus naunau",
        "text_unlimited_nvarchar": "余余余余余",
        "text_unlimited_text": "muspus naunau",
        "text_unlimited_ntext": "余余余余余"
    })
}

const FIND_MANY_TYPES_QUERY: &str = indoc!(
    r##"
    query {
        findManytypes {
            "numeric_integer_tinyint",
            "numeric_integer_smallint",
            "numeric_integer_int",
            "numeric_integer_bigint",
            "numeric_floating_decimal",
            "numeric_floating_numeric",
            "numeric_floating_float",
            "numeric_floating_double",
            "numeric_floating_real",
            "numeric_floating_money",
            "numeric_floating_smallmoney",
            "numeric_boolean",
            "date_date",
            "date_time",
            "date_datetime",
            "date_datetime2",
            "date_smalldatetime",
            "date_datetimeoffset",
            "text_limited_char",
            "text_limited_varchar",
            "text_limited_nvarchar",
            "text_unlimited_varchar",
            "text_unlimited_nvarchar",
            "text_unlimited_text",
            "text_unlimited_ntext"
        }
    }
    "##
);

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn types_roundtrip(api: &TestApi) -> TestResult {
    api.execute_sql(&create_types_table_sql()).await?;

    let (datamodel, engine) = api.introspect_and_start_query_engine().await?;

    datamodel.assert_model("types", |model| {
        model
            .assert_field_type("numeric_integer_tinyint", ScalarType::Int)?
            .assert_field_type("numeric_integer_smallint", ScalarType::Int)?
            .assert_field_type("numeric_integer_int", ScalarType::Int)?
            .assert_field_type("numeric_integer_bigint", ScalarType::Int)?
            .assert_field_type("numeric_floating_decimal", ScalarType::Float)?
            .assert_field_type("numeric_floating_float", ScalarType::Float)?
            .assert_field_type("numeric_floating_double", ScalarType::Float)?
            .assert_field_type("numeric_floating_real", ScalarType::Float)?
            .assert_field_type("numeric_floating_money", ScalarType::Float)?
            .assert_field_type("numeric_floating_smallmoney", ScalarType::Float)?
            .assert_field_type("numeric_boolean", ScalarType::Boolean)?
            .assert_field_type("date_date", ScalarType::DateTime)?
            .assert_field_type("date_time", ScalarType::DateTime)?
            .assert_field_type("date_datetime", ScalarType::DateTime)?
            .assert_field_type("date_datetime2", ScalarType::DateTime)?
            .assert_field_type("date_smalldatetime", ScalarType::DateTime)?
            .assert_field_type("date_datetimeoffset", ScalarType::DateTime)?
            .assert_field_type("text_limited_char", ScalarType::String)?
            .assert_field_type("text_limited_varchar", ScalarType::String)?
            .assert_field_type("text_limited_nvarchar", ScalarType::String)?
            .assert_field_type("text_unlimited_varchar", ScalarType::String)?
            .assert_field_type("text_unlimited_nvarchar", ScalarType::String)?
            .assert_field_type("text_unlimited_text", ScalarType::String)?
            .assert_field_type("text_unlimited_ntext", ScalarType::String)
    })?;

    // Write the values.
    {
        let write = CREATE_ONE_TYPES_QUERY;

        let write_response = engine.request(write).await;

        let expected_write_response = json!({
            "data": {
                "createOnetypes": {
                    "id": 1,
                }
            }
        });

        assert_eq!(write_response, expected_write_response);
    }

    // Read the values back.
    {
        let read_response = engine.request(FIND_MANY_TYPES_QUERY).await;

        let expected_read_response = json!({
            "data": {
                "findManytypes": [
                    expected_create_one_types_result(),
                ],
            },
        });

        assert_eq!(read_response, expected_read_response);
    }

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn floats_do_not_lose_precision(api: &TestApi) -> TestResult {
    api.execute_sql(&create_types_table_sql()).await?;

    let (_datamodel, engine) = api.introspect_and_start_query_engine().await?;

    let write = indoc! {
        "
        mutation {
            createOnetypes(
                data: {
                    numeric_floating_float: 6.4
                    numeric_floating_double: 6.4
                    numeric_floating_decimal: 6.4
                    numeric_floating_numeric: 6.4
                    numeric_floating_real: 6.4
                    numeric_floating_money: 6.4
                    numeric_floating_smallmoney: 6.4
                }
            ) {
                id
                numeric_floating_float
                numeric_floating_decimal
                numeric_floating_numeric
                numeric_floating_real
                numeric_floating_money
                numeric_floating_smallmoney
            }
        }
        "
    };

    let write_response = engine.request(write).await;

    let expected_write_response = json!({
        "data": {
            "createOnetypes": {
                "id": 1,
                "numeric_floating_float": 6.4,
                "numeric_floating_double": 6.4,
                "numeric_floating_decimal": 6.4,
                "numeric_floating_numeric": 6.4,
                "numeric_floating_real": 6.4,
                "numeric_floating_money": 6.4,
                "numeric_floating_smallmoney": 6.4,
            }
        }
    });

    assert_eq!(write_response, expected_write_response);

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn all_types_work_as_filter(api: &TestApi) -> TestResult {
    api.execute_sql(&create_types_table_sql()).await?;

    let (_datamodel, engine) = api.introspect_and_start_query_engine().await?;

    engine.request(CREATE_ONE_TYPES_QUERY).await;

    let query = "
        query {
            findManytypes(
                where: {
                    numeric_integer_tinyint: { equals: 12 }
                    numeric_integer_smallint: { equals: 350 }
                    numeric_integer_int: { equals: 9002 }
                    numeric_integer_bigint: { equals: 30000 }
                    numeric_floating_decimal: { equals: 3.14 }
                    numeric_floating_numeric: { equals: 3.14 }
                    numeric_floating_float: { equals: -32.0 }
                    numeric_floating_double: { equals: 0.14 }
                    numeric_floating_real: { equals: 12.12 }
                    numeric_floating_money: { equals: 12.12 }
                    numeric_floating_smallmoney: { equals: 12.12 }
                    numeric_boolean: { equals: true }
                    date_date: { equals: \"2020-02-27T00:00:00Z\" }
                    date_time: { equals: \"1970-01-01T16:20:20Z\" }
                    date_datetime: { equals: \"2020-02-27T19:10:22Z\" }
                    date_datetime2: { equals: \"2020-02-27T19:10:22Z\" }
                    date_smalldatetime: { equals: \"2020-02-27T19:10:22Z\" }
                    date_datetimeoffset: { equals: \"2020-02-27T19:10:22Z\" }
                    text_limited_char: { equals: \"abcdefghij\" }
                    text_limited_varchar: { equals: \"muspus naunau\" }
                    text_limited_nvarchar: { equals: \"余余余余余\" }
                    text_unlimited_varchar: { equals: \"muspus naunau\" }
                    text_unlimited_nvarchar: { equals: \"余余余余余\" }
                    text_unlimited_text: { equals: \"muspus naunau\" }
                    text_unlimited_ntext: { equals: \"余余余余余\" }
                }
            ) {
                id
            }
        }
    ";

    let response = engine.request(query).await;

    let expected_json = json!({ "data": { "findManytypes": [{ "id": 1 }] } });

    assert_eq!(response, expected_json);

    Ok(())
}
