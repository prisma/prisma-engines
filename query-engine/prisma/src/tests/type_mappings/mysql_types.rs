use super::test_api::*;
use datamodel::dml::ScalarType;
use indoc::indoc;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_macros::*;

const CREATE_TYPES_TABLE: &str = indoc! {
    r##"
    CREATE TABLE `types` (
        `id` int(11) NOT NULL AUTO_INCREMENT,
        `numeric_integer_tinyint` tinyint(4),
        `numeric_integer_smallint` smallint(6),
        `numeric_integer_int` int(11),
        `numeric_integer_bigint` bigint(20),
        `numeric_floating_decimal` decimal(10,2),
        `numeric_floating_float` float,
        `numeric_fixed_double` double,
        `numeric_fixed_real` double,
        `numeric_bit` bit(64),
        `numeric_boolean` tinyint(1),
        `date_date` date,
        `date_datetime` datetime,
        `date_timestamp` timestamp null DEFAULT null,
        `date_time` time,
        `date_year` year(4),
        `string_char` char(255),
        `string_varchar` varchar(255),
        `string_text_tinytext` tinytext,
        `string_text_text` text,
        `string_text_mediumtext` mediumtext,
        `string_text_longtext` longtext,
        `string_binary_binary` binary(20),
        `string_binary_varbinary` varbinary(255),
        `string_blob_tinyblob` tinyblob,
        `string_blob_mediumblob` mediumblob,
        `string_blob_blob` blob,
        `string_blob_longblob` longblob,
        `string_enum` enum('pollicle_dogs','jellicle_cats'),
        `string_set` set('a','b','c'),
        `spatial_geometry` geometry,
        `spatial_point` point,
        `spatial_linestring` linestring,
        `spatial_polygon` polygon,
        `spatial_multipoint` multipoint,
        `spatial_multilinestring` multilinestring,
        `spatial_multipolygon` multipolygon,
        `spatial_geometrycollection` geometrycollection,
        `json` json,

        PRIMARY KEY (`id`)
      ) ENGINE=InnoDB DEFAULT CHARSET=latin1;
    "##
};

#[test_each_connector(tags("mysql"))]
async fn mysql_types_roundtrip(api: &TestApi) -> TestResult {
    api.execute_sql(CREATE_TYPES_TABLE).await?;

    let (datamodel, engine) = api.introspect_and_start_query_engine().await?;

    datamodel.assert_model("types", |model| {
        model
            .assert_field_type("numeric_integer_tinyint", ScalarType::Int)?
            .assert_field_type("numeric_integer_smallint", ScalarType::Int)?
            .assert_field_type("numeric_integer_int", ScalarType::Int)?
            .assert_field_type("numeric_integer_bigint", ScalarType::Int)?
            .assert_field_type("numeric_floating_decimal", ScalarType::Float)?
            .assert_field_type("numeric_floating_float", ScalarType::Float)?
            .assert_field_type("numeric_fixed_double", ScalarType::Float)?
            .assert_field_type("numeric_fixed_real", ScalarType::Float)?
            .assert_field_type("numeric_bit", ScalarType::Int)?
            .assert_field_type("numeric_boolean", ScalarType::Boolean)?
            .assert_field_type("date_date", ScalarType::DateTime)?
            .assert_field_type("date_datetime", ScalarType::DateTime)?
            .assert_field_type("date_timestamp", ScalarType::DateTime)?
            .assert_field_type("date_time", ScalarType::DateTime)?
            .assert_field_type("date_year", ScalarType::Int)?
            .assert_field_type("string_char", ScalarType::String)?
            .assert_field_type("string_varchar", ScalarType::String)?
            .assert_field_type("string_text_tinytext", ScalarType::String)?
            .assert_field_type("string_text_text", ScalarType::String)?
            .assert_field_type("string_text_mediumtext", ScalarType::String)?
            .assert_field_type("string_text_longtext", ScalarType::String)?
            .assert_field_type("string_binary_binary", ScalarType::String)?
            .assert_field_type("string_blob_tinyblob", ScalarType::String)?
            .assert_field_type("string_blob_mediumblob", ScalarType::String)?
            .assert_field_type("string_blob_blob", ScalarType::String)?
            .assert_field_type("string_blob_longblob", ScalarType::String)?
            .assert_field_enum_type("string_enum", "types_string_enum")?
            .assert_field_type("string_set", ScalarType::String)?
            .assert_field_type("spatial_geometry", ScalarType::String)?
            .assert_field_type("spatial_point", ScalarType::String)?
            .assert_field_type("spatial_linestring", ScalarType::String)?
            .assert_field_type("spatial_polygon", ScalarType::String)?
            .assert_field_type("spatial_multipoint", ScalarType::String)?
            .assert_field_type("spatial_multilinestring", ScalarType::String)?
            .assert_field_type("spatial_multipolygon", ScalarType::String)?
            .assert_field_type("spatial_geometrycollection", ScalarType::String)?
            .assert_field_type("json", ScalarType::String)
    })?;

    // Write the values.
    {
        let write = indoc! {
            "
            mutation {
                createOnetypes(
                    data: {
                        numeric_integer_tinyint: 12,
                        numeric_integer_smallint: 350,
                        numeric_integer_int: 9002,
                        numeric_integer_bigint: 30000,
                        numeric_floating_decimal: 3.14
                        numeric_floating_float: -32.0
                        numeric_fixed_double: 0.14
                        numeric_fixed_real: 12.12
                        numeric_bit: 4
                        numeric_boolean: true
                        date_date: \"2020-02-27T00:00:00Z\"
                        date_datetime: \"2020-02-27T19:10:22Z\"
                        date_timestamp: \"2020-02-27T19:11:22Z\"
                        date_time: \"2020-02-20T12:50:01Z\"
                        date_year: 2012
                        string_char: \"make dolphins easy\"
                        string_varchar: \"dolphins of varying characters\"
                        string_text_tinytext: \"tiny dolphins\"
                        string_text_text: \"dolphins\"
                        string_text_mediumtext: \"medium dolphins\"
                        string_text_longtext: \"long dolphins\"
                        string_binary_binary: \"hello 2020\"
                        string_blob_tinyblob: \"smol blob\"
                        string_blob_mediumblob: \"average blob\"
                        string_blob_blob: \"very average blob\"
                        string_blob_longblob: \"loong looooong bloooooooob\"
                        string_enum: \"jellicle_cats\"
                        json: \"{\\\"name\\\": null}\"
                    }
                ) { id }
            }
            "
        };

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
        let read = indoc! {
            "
            query {
                findManytypes {
                    numeric_integer_tinyint
                    numeric_integer_smallint
                    numeric_integer_int
                    numeric_integer_bigint
                    numeric_floating_decimal
                    numeric_floating_float
                    numeric_fixed_double
                    numeric_fixed_real
                    numeric_bit
                    numeric_boolean
                    date_date
                    date_datetime
                    date_timestamp
                    date_time
                    date_year
                    string_char
                    string_varchar
                    string_text_tinytext
                    string_text_text
                    string_text_mediumtext
                    string_text_longtext
                    string_binary_binary
                    string_blob_tinyblob
                    string_blob_mediumblob
                    string_blob_blob
                    string_blob_longblob
                    string_enum
                    # omitting spatial/geometry types
                    json
                }
            }
            "
        };

        let read_response = engine.request(read).await;

        let expected_read_response = json!({
            "data": {
                "findManytypes": [
                    {
                        "numeric_integer_tinyint": 12,
                        "numeric_integer_smallint": 350,
                        "numeric_integer_int": 9002,
                        "numeric_integer_bigint": 30000,
                        "numeric_floating_decimal": 3.14,
                        "numeric_floating_float": -32.0,
                        "numeric_fixed_double": 0.14,
                        "numeric_fixed_real": 12.12,
                        "numeric_bit": 4,
                        "numeric_boolean": true,
                        "date_date": "2020-02-27T00:00:00.000Z",
                        "date_datetime": "2020-02-27T19:10:22.000Z",
                        "date_timestamp": "2020-02-27T19:11:22.000Z",
                        "date_time": "1970-01-01T12:50:01.000Z",
                        "date_year": 2012,
                        "string_char": "make dolphins easy",
                        "string_varchar": "dolphins of varying characters",
                        "string_text_tinytext": "tiny dolphins",
                        "string_text_text": "dolphins",
                        "string_text_mediumtext": "medium dolphins",
                        "string_text_longtext": "long dolphins",
                        "string_binary_binary": "hello 2020\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}",
                        "string_blob_tinyblob": "smol blob",
                        "string_blob_mediumblob": "average blob",
                        "string_blob_blob": "very average blob",
                        "string_blob_longblob": "loong looooong bloooooooob",
                        "string_enum": "jellicle_cats",
                        "json": "{\"name\": null}",
                    },
                ]
            },
        });

        assert_eq!(read_response, expected_read_response);
    }

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn mysql_bit_columns_are_properly_mapped_to_signed_integers(api: &TestApi) -> TestResult {
    api.execute_sql(CREATE_TYPES_TABLE).await?;

    let (_datamodel, engine) = api.introspect_and_start_query_engine().await?;

    let write = indoc! {
        "
        mutation {
            createOnetypes(
                data: {
                    numeric_bit: -12
                }
            ) { id numeric_bit }
        }
        "
    };

    let write_response = engine.request(write).await;

    let expected_write_response = json!({
        "data": {
            "createOnetypes": {
                "id": 1,
                "numeric_bit": -12,
            }
        }
    });

    assert_eq!(write_response, expected_write_response);

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn mysql_floats_do_not_lose_precision(api: &TestApi) -> TestResult {
    api.execute_sql(CREATE_TYPES_TABLE).await?;

    let (_datamodel, engine) = api.introspect_and_start_query_engine().await?;

    let write = indoc! {
        "
        mutation {
            createOnetypes(
                data: {
                    numeric_floating_float: 6.4
                    numeric_floating_decimal: 6.4
                }
            ) {
                id
                numeric_floating_float
                numeric_floating_decimal
            }
        }
        "
    };

    let write_response = engine.request(write).await;

    let expected_write_response = json!({
        "data": {
            "createOnetypes": {
                "id": 1,
                // This is what we expect from a f64 -> f32 -> f64 conversion.
                "numeric_floating_float": 6.400000095367432,
                "numeric_floating_decimal": 6.4,
            }
        }
    });

    assert_eq!(write_response, expected_write_response);

    Ok(())
}

#[test_each_connector(tags("mysql"), log = "debug")]
async fn all_mysql_identifier_types_work(api: &TestApi) -> TestResult {
    let identifier_types = &[
        ("tinyint", "12", ""),
        ("smallint", "350", ""),
        ("int", "9002", ""),
        ("bigint", "30000", ""),
        ("decimal(4, 2)", "3.1", ""),
        // ("float", "2.8", ""),
        ("double", "0.1", ""),
        ("real", "12.1", ""),
        ("bit(32)", "4", ""),
        ("boolean", "true", ""),
        ("date", "\"2020-02-27T00:00:00.000Z\"", ""),
        ("datetime", "\"2020-02-27T19:10:22.000Z\"", ""),
        ("timestamp", "\"2020-02-27T19:11:22.000Z\"", ""),
        // ("time", "\"1970-01-01T12:50:01.000Z\"", ""),
        ("year", "2091", ""),
        ("char(18)", "\"make dolphins easy\"", ""),
        ("varchar(200)", "\"dolphins of varying characters\"", ""),
        ("tinytext", "\"tiny dolphins\"", "(20)"),
        ("text", "\"dolphins\"", "(100)"),
        ("mediumtext", "\"medium dolphins\"", "(100)"),
        ("longtext", "\"long dolphins\"", "(100)"),
        (
            "enum('pollicle_dogs','jellicle_cats')",
            "\"jellicle_cats\"",
            "",
        ),
        // ("json", "\"{\\\"name\\\": null}\"", ""),
    ];

    let drop_table = r#"DROP TABLE IF EXISTS `pk_test`"#;

    for (identifier_type, identifier_value, prefix) in identifier_types {
        for index_type in &["PRIMARY KEY", "CONSTRAINT UNIQUE INDEX"] {
            let create_pk_table = format!(
                r#"CREATE TABLE `pk_test` (id {} NOT NULL, {} (id{}))"#,
                identifier_type, index_type, prefix
            );
            api.execute_sql(drop_table).await?;
            api.execute_sql(&create_pk_table).await?;

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
                identifier_value
            );

            let response = engine.request(query).await;

            let expected_response = format!(
                r#"{{"data":{{"createOnepk_test":{{"id":{}}}}}}}"#,
                identifier_value
            );
            assert_eq!(response.to_string(), expected_response);
        }
    }

    Ok(())
}
