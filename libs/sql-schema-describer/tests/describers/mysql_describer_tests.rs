use crate::test_api::*;
use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

#[test_connector(tags(Mysql))]
fn views_can_be_described(api: TestApi) {
    let sql = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
        CREATE VIEW ab AS
            SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
    "#;
    api.raw_cmd(sql);

    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    let expected_sql = format!(
        "select `{0}`.`a`.`a_id` AS `a_id` from `{0}`.`a` union all select `{0}`.`b`.`b_id` AS `b_id` from `{0}`.`b`",
        api.db_name()
    );

    assert_eq!("ab", &view.name);
    assert_eq!(expected_sql, view.definition.unwrap());
}

#[test_connector(tags(Mysql))]
fn procedures_can_be_described(api: TestApi) {
    let sql = format!(
        r#"
        CREATE PROCEDURE {}.foo (OUT res INT) SELECT 1 INTO res
        "#,
        api.db_name()
    );

    api.raw_cmd(&sql);
    let result = api.describe();
    let procedure = result.get_procedure("foo").unwrap();

    assert_eq!("foo", &procedure.name);
    assert_eq!(Some("SELECT 1 INTO res"), procedure.definition.as_deref());
}

#[test_connector(tags(Mysql), exclude(Mysql8, Mysql56, Mariadb))]
fn all_mysql_column_types_must_work(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::primary());
        t.add_column("int_col", types::custom("int"));
        t.add_column("smallint_col", types::custom("smallint"));
        t.add_column("tinyint4_col", types::custom("tinyint(4)"));
        t.add_column("tinyint1_col", types::custom("tinyint(1)"));
        t.add_column("mediumint_col", types::custom("mediumint"));
        t.add_column("bigint_col", types::custom("bigint"));
        t.add_column("decimal_col", types::custom("decimal"));
        t.add_column("numeric_col", types::custom("numeric"));
        t.add_column("float_col", types::custom("float"));
        t.add_column("double_col", types::custom("double"));
        t.add_column("date_col", types::custom("date"));
        t.add_column("time_col", types::custom("time"));
        t.add_column("datetime_col", types::custom("datetime"));
        t.add_column("timestamp_col", types::custom("timestamp"));
        t.add_column("year_col", types::custom("year"));
        t.add_column("char_col", types::custom("char"));
        t.add_column("varchar_col", types::custom("varchar(255)"));
        t.add_column("text_col", types::custom("text"));
        t.add_column("tinytext_col", types::custom("tinytext"));
        t.add_column("mediumtext_col", types::custom("mediumtext"));
        t.add_column("longtext_col", types::custom("longtext"));
        t.add_column("enum_col", types::custom("enum('a', 'b')"));
        t.add_column("set_col", types::custom("set('a', 'b')"));
        t.add_column("binary_col", types::custom("binary"));
        t.add_column("varbinary_col", types::custom("varbinary(255)"));
        t.add_column("blob_col", types::custom("blob"));
        t.add_column("tinyblob_col", types::custom("tinyblob"));
        t.add_column("mediumblob_col", types::custom("mediumblob"));
        t.add_column("longblob_col", types::custom("longblob"));
        t.add_column("geometry_col", types::custom("geometry"));
        t.add_column("point_col", types::custom("point"));
        t.add_column("linestring_col", types::custom("linestring"));
        t.add_column("polygon_col", types::custom("polygon"));
        t.add_column("multipoint_col", types::custom("multipoint"));
        t.add_column("multilinestring_col", types::custom("multilinestring"));
        t.add_column("multipolygon_col", types::custom("multipolygon"));
        t.add_column("geometrycollection_col", types::custom("geometrycollection"));
        t.add_column("json_col", types::custom("json"));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    values: [
                        "a",
                        "b",
                    ],
                },
            ],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "int(11)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int(11)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "smallint(6)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint4_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(4)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint1_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(1)",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumint_col",
                        tpe: ColumnType {
                            full_data_type: "mediumint(9)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "bigint(20)",
                            family: BigInt,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "BigInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "decimal_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "float",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Float",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "double",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Double",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "date_col",
                        tpe: ColumnType {
                            full_data_type: "date",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Date",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "time_col",
                        tpe: ColumnType {
                            full_data_type: "time",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Time": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetime_col",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "DateTime": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "timestamp_col",
                        tpe: ColumnType {
                            full_data_type: "timestamp",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timestamp": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Now,
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "year_col",
                        tpe: ColumnType {
                            full_data_type: "year(4)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Year",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "char(1)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Char": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varchar_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "text_col",
                        tpe: ColumnType {
                            full_data_type: "text",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Text",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinytext_col",
                        tpe: ColumnType {
                            full_data_type: "tinytext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumtext_col",
                        tpe: ColumnType {
                            full_data_type: "mediumtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longtext_col",
                        tpe: ColumnType {
                            full_data_type: "longtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "enum_col",
                        tpe: ColumnType {
                            full_data_type: "enum('a','b')",
                            family: Enum(
                                "User_enum_col",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "set_col",
                        tpe: ColumnType {
                            full_data_type: "set('a','b')",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "binary(1)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Binary": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varbinary_col",
                        tpe: ColumnType {
                            full_data_type: "varbinary(255)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBinary": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "blob_col",
                        tpe: ColumnType {
                            full_data_type: "blob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Blob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyblob_col",
                        tpe: ColumnType {
                            full_data_type: "tinyblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumblob_col",
                        tpe: ColumnType {
                            full_data_type: "mediumblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longblob_col",
                        tpe: ColumnType {
                            full_data_type: "longblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometry_col",
                        tpe: ColumnType {
                            full_data_type: "geometry",
                            family: Unsupported(
                                "geometry",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "point_col",
                        tpe: ColumnType {
                            full_data_type: "point",
                            family: Unsupported(
                                "point",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "linestring_col",
                        tpe: ColumnType {
                            full_data_type: "linestring",
                            family: Unsupported(
                                "linestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "polygon_col",
                        tpe: ColumnType {
                            full_data_type: "polygon",
                            family: Unsupported(
                                "polygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipoint_col",
                        tpe: ColumnType {
                            full_data_type: "multipoint",
                            family: Unsupported(
                                "multipoint",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multilinestring_col",
                        tpe: ColumnType {
                            full_data_type: "multilinestring",
                            family: Unsupported(
                                "multilinestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipolygon_col",
                        tpe: ColumnType {
                            full_data_type: "multipolygon",
                            family: Unsupported(
                                "multipolygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometrycollection_col",
                        tpe: ColumnType {
                            full_data_type: "geometrycollection",
                            family: Unsupported(
                                "geometrycollection",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "json",
                            family: Json,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Json",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mariadb))]
fn all_mariadb_column_types_must_work(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::primary());
        t.add_column("int_col", types::custom("int"));
        t.add_column("smallint_col", types::custom("smallint"));
        t.add_column("tinyint4_col", types::custom("tinyint(4)"));
        t.add_column("tinyint1_col", types::custom("tinyint(1)"));
        t.add_column("mediumint_col", types::custom("mediumint"));
        t.add_column("bigint_col", types::custom("bigint"));
        t.add_column("decimal_col", types::custom("decimal"));
        t.add_column("numeric_col", types::custom("numeric"));
        t.add_column("float_col", types::custom("float"));
        t.add_column("double_col", types::custom("double"));
        t.add_column("date_col", types::custom("date"));
        t.add_column("time_col", types::custom("time"));
        t.add_column("datetime_col", types::custom("datetime"));
        t.add_column("timestamp_col", types::custom("timestamp"));
        t.add_column("year_col", types::custom("year"));
        t.add_column("char_col", types::custom("char"));
        t.add_column("varchar_col", types::custom("varchar(255)"));
        t.add_column("text_col", types::custom("text"));
        t.add_column("tinytext_col", types::custom("tinytext"));
        t.add_column("mediumtext_col", types::custom("mediumtext"));
        t.add_column("longtext_col", types::custom("longtext"));
        t.add_column("enum_col", types::custom("enum('a', 'b')"));
        t.add_column("set_col", types::custom("set('a', 'b')"));
        t.add_column("binary_col", types::custom("binary"));
        t.add_column("varbinary_col", types::custom("varbinary(255)"));
        t.add_column("blob_col", types::custom("blob"));
        t.add_column("tinyblob_col", types::custom("tinyblob"));
        t.add_column("mediumblob_col", types::custom("mediumblob"));
        t.add_column("longblob_col", types::custom("longblob"));
        t.add_column("geometry_col", types::custom("geometry"));
        t.add_column("point_col", types::custom("point"));
        t.add_column("linestring_col", types::custom("linestring"));
        t.add_column("polygon_col", types::custom("polygon"));
        t.add_column("multipoint_col", types::custom("multipoint"));
        t.add_column("multilinestring_col", types::custom("multilinestring"));
        t.add_column("multipolygon_col", types::custom("multipolygon"));
        t.add_column("geometrycollection_col", types::custom("geometrycollection"));
        t.add_column("json_col", types::custom("json"));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    values: [
                        "a",
                        "b",
                    ],
                },
            ],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "int(11)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int(11)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "smallint(6)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint4_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(4)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint1_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(1)",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumint_col",
                        tpe: ColumnType {
                            full_data_type: "mediumint(9)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "bigint(20)",
                            family: BigInt,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "BigInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "decimal_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "float",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Float",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "double",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Double",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "date_col",
                        tpe: ColumnType {
                            full_data_type: "date",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Date",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "time_col",
                        tpe: ColumnType {
                            full_data_type: "time",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Time": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetime_col",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "DateTime": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "timestamp_col",
                        tpe: ColumnType {
                            full_data_type: "timestamp",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timestamp": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Now,
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "year_col",
                        tpe: ColumnType {
                            full_data_type: "year(4)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Year",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "char(1)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Char": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varchar_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "text_col",
                        tpe: ColumnType {
                            full_data_type: "text",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Text",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinytext_col",
                        tpe: ColumnType {
                            full_data_type: "tinytext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumtext_col",
                        tpe: ColumnType {
                            full_data_type: "mediumtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longtext_col",
                        tpe: ColumnType {
                            full_data_type: "longtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "enum_col",
                        tpe: ColumnType {
                            full_data_type: "enum('a','b')",
                            family: Enum(
                                "User_enum_col",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "set_col",
                        tpe: ColumnType {
                            full_data_type: "set('a','b')",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "binary(1)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Binary": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varbinary_col",
                        tpe: ColumnType {
                            full_data_type: "varbinary(255)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBinary": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "blob_col",
                        tpe: ColumnType {
                            full_data_type: "blob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Blob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyblob_col",
                        tpe: ColumnType {
                            full_data_type: "tinyblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumblob_col",
                        tpe: ColumnType {
                            full_data_type: "mediumblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longblob_col",
                        tpe: ColumnType {
                            full_data_type: "longblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometry_col",
                        tpe: ColumnType {
                            full_data_type: "geometry",
                            family: Unsupported(
                                "geometry",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "point_col",
                        tpe: ColumnType {
                            full_data_type: "point",
                            family: Unsupported(
                                "point",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "linestring_col",
                        tpe: ColumnType {
                            full_data_type: "linestring",
                            family: Unsupported(
                                "linestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "polygon_col",
                        tpe: ColumnType {
                            full_data_type: "polygon",
                            family: Unsupported(
                                "polygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipoint_col",
                        tpe: ColumnType {
                            full_data_type: "multipoint",
                            family: Unsupported(
                                "multipoint",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multilinestring_col",
                        tpe: ColumnType {
                            full_data_type: "multilinestring",
                            family: Unsupported(
                                "multilinestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipolygon_col",
                        tpe: ColumnType {
                            full_data_type: "multipolygon",
                            family: Unsupported(
                                "multipolygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometrycollection_col",
                        tpe: ColumnType {
                            full_data_type: "geometrycollection",
                            family: Unsupported(
                                "geometrycollection",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "longtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql8))]
fn all_mysql_8_column_types_must_work(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::primary());
        t.add_column("int_col", types::custom("int"));
        t.add_column("smallint_col", types::custom("smallint"));
        t.add_column("tinyint4_col", types::custom("tinyint(4)"));
        t.add_column("tinyint1_col", types::custom("tinyint(1)"));
        t.add_column("mediumint_col", types::custom("mediumint"));
        t.add_column("bigint_col", types::custom("bigint"));
        t.add_column("decimal_col", types::custom("decimal"));
        t.add_column("numeric_col", types::custom("numeric"));
        t.add_column("float_col", types::custom("float"));
        t.add_column("double_col", types::custom("double"));
        t.add_column("date_col", types::custom("date"));
        t.add_column("time_col", types::custom("time"));
        t.add_column("datetime_col", types::custom("datetime"));
        t.add_column("timestamp_col", types::custom("timestamp"));
        t.add_column("year_col", types::custom("year"));
        t.add_column("char_col", types::custom("char"));
        t.add_column("varchar_col", types::custom("varchar(255)"));
        t.add_column("text_col", types::custom("text"));
        t.add_column("tinytext_col", types::custom("tinytext"));
        t.add_column("mediumtext_col", types::custom("mediumtext"));
        t.add_column("longtext_col", types::custom("longtext"));
        t.add_column("enum_col", types::custom("enum('a', 'b')"));
        t.add_column("set_col", types::custom("set('a', 'b')"));
        t.add_column("binary_col", types::custom("binary"));
        t.add_column("varbinary_col", types::custom("varbinary(255)"));
        t.add_column("blob_col", types::custom("blob"));
        t.add_column("tinyblob_col", types::custom("tinyblob"));
        t.add_column("mediumblob_col", types::custom("mediumblob"));
        t.add_column("longblob_col", types::custom("longblob"));
        t.add_column("geometry_col", types::custom("geometry"));
        t.add_column("point_col", types::custom("point"));
        t.add_column("linestring_col", types::custom("linestring"));
        t.add_column("polygon_col", types::custom("polygon"));
        t.add_column("multipoint_col", types::custom("multipoint"));
        t.add_column("multilinestring_col", types::custom("multilinestring"));
        t.add_column("multipolygon_col", types::custom("multipolygon"));
        t.add_column("geometrycollection_col", types::custom("geometrycollection"));
        t.add_column("json_col", types::custom("json"));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    values: [
                        "a",
                        "b",
                    ],
                },
            ],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "smallint",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint4_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint1_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(1)",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumint_col",
                        tpe: ColumnType {
                            full_data_type: "mediumint",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "bigint",
                            family: BigInt,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "BigInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "decimal_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "float",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Float",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "double",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Double",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "date_col",
                        tpe: ColumnType {
                            full_data_type: "date",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Date",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "time_col",
                        tpe: ColumnType {
                            full_data_type: "time",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Time": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetime_col",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "DateTime": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "timestamp_col",
                        tpe: ColumnType {
                            full_data_type: "timestamp",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timestamp": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "year_col",
                        tpe: ColumnType {
                            full_data_type: "year",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Year",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "char(1)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Char": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varchar_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "text_col",
                        tpe: ColumnType {
                            full_data_type: "text",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Text",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinytext_col",
                        tpe: ColumnType {
                            full_data_type: "tinytext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumtext_col",
                        tpe: ColumnType {
                            full_data_type: "mediumtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longtext_col",
                        tpe: ColumnType {
                            full_data_type: "longtext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "enum_col",
                        tpe: ColumnType {
                            full_data_type: "enum('a','b')",
                            family: Enum(
                                "User_enum_col",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "set_col",
                        tpe: ColumnType {
                            full_data_type: "set('a','b')",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "binary(1)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Binary": Number(
                                        1,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varbinary_col",
                        tpe: ColumnType {
                            full_data_type: "varbinary(255)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBinary": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "blob_col",
                        tpe: ColumnType {
                            full_data_type: "blob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Blob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyblob_col",
                        tpe: ColumnType {
                            full_data_type: "tinyblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "mediumblob_col",
                        tpe: ColumnType {
                            full_data_type: "mediumblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "MediumBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "longblob_col",
                        tpe: ColumnType {
                            full_data_type: "longblob",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "LongBlob",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometry_col",
                        tpe: ColumnType {
                            full_data_type: "geometry",
                            family: Unsupported(
                                "geometry",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "point_col",
                        tpe: ColumnType {
                            full_data_type: "point",
                            family: Unsupported(
                                "point",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "linestring_col",
                        tpe: ColumnType {
                            full_data_type: "linestring",
                            family: Unsupported(
                                "linestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "polygon_col",
                        tpe: ColumnType {
                            full_data_type: "polygon",
                            family: Unsupported(
                                "polygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipoint_col",
                        tpe: ColumnType {
                            full_data_type: "multipoint",
                            family: Unsupported(
                                "multipoint",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multilinestring_col",
                        tpe: ColumnType {
                            full_data_type: "multilinestring",
                            family: Unsupported(
                                "multilinestring",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "multipolygon_col",
                        tpe: ColumnType {
                            full_data_type: "multipolygon",
                            family: Unsupported(
                                "multipolygon",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "geometrycollection_col",
                        tpe: ColumnType {
                            full_data_type: "geomcollection",
                            family: Unsupported(
                                "geomcollection",
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "json",
                            family: Json,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Json",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql))]
fn mysql_foreign_key_on_delete_must_be_handled(api: TestApi) {
    // NB: We don't test the SET DEFAULT variety since it isn't supported on InnoDB and will
    // just cause an error
    let sql = format!(
        "CREATE TABLE `{0}`.City (id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY);
         CREATE TABLE `{0}`.User (
            id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
            city INTEGER, FOREIGN KEY(city) REFERENCES City (id) ON DELETE NO ACTION,
            city_cascade INTEGER, FOREIGN KEY(city_cascade) REFERENCES City (id) ON DELETE CASCADE,
            city_restrict INTEGER, FOREIGN KEY(city_restrict) REFERENCES City (id) ON DELETE RESTRICT,
            city_set_null INTEGER, FOREIGN KEY(city_set_null) REFERENCES City (id) ON DELETE SET NULL
        )",
        api.db_name()
    );
    api.raw_cmd(&sql);

    api.describe().assert_table("User", |t| {
        t.assert_column("id", |id| id.assert_type_is_int())
            .assert_column("city", |c| c.assert_type_is_int())
            .assert_column("city_cascade", |c| c.assert_type_is_int())
            .assert_column("city_restrict", |c| c.assert_type_is_int())
            .assert_column("city_set_null", |c| c.assert_type_is_int())
            .assert_index_on_columns(&["city"], |idx| idx.assert_is_not_unique())
            .assert_index_on_columns(&["city_cascade"], |idx| idx.assert_is_not_unique())
            .assert_index_on_columns(&["city_restrict"], |idx| idx.assert_is_not_unique())
            .assert_index_on_columns(&["city_set_null"], |idx| idx.assert_is_not_unique())
            .assert_foreign_key_on_columns(&["city"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::NoAction)
            })
            .assert_foreign_key_on_columns(&["city_cascade"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_foreign_key_on_columns(&["city_restrict"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Restrict)
            })
            .assert_foreign_key_on_columns(&["city_set_null"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetNull)
            })
    });
}

#[test_connector(tags(Mysql8))]
fn mysql_multi_field_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let result = api.describe();
    result.assert_table("Employee", |t| {
        t.assert_index_on_columns(&["name", "age"], |idx| idx.assert_name("age_and_name_index"))
    });
}

#[test_connector(tags(Mysql), exclude(Mysql8))]
fn old_mysql_multi_field_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let result = api.describe();
    result.assert_table("Employee", |t| {
        t.assert_index_on_columns(&["name", "age"], |idx| idx.assert_name("age_and_name_index"))
    });
}

#[test_connector(tags(Mysql))]
fn mysql_join_table_unique_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());

    migration.create_table("Cat", move |t| {
        t.add_column("id", types::primary());
        t.add_column("name", types::text());
    });

    migration.create_table("Human", move |t| {
        t.add_column("id", types::primary());
        t.add_column("name", types::text());
    });

    migration.create_table("CatToHuman", move |t| {
        t.add_column("cat", types::foreign("Cat", "id").nullable(true));
        t.add_column("human", types::foreign("Human", "id").nullable(true));
        t.add_column("relationship", types::text());
        t.add_index("cat_and_human_index", types::index(vec!["cat", "human"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);

    api.describe().assert_table("CatToHuman", |t| {
        t.assert_index_on_columns(&["cat", "human"], |idx| {
            idx.assert_name("cat_and_human_index").assert_is_unique()
        })
    });
}

// When multiple databases exist on a mysql instance, and they share names for foreign key
// constraints, introspecting one database should not yield constraints from the other.
#[test_connector(tags(Mysql))]
fn constraints_from_other_databases_should_not_be_introspected(api: TestApi) {
    api.block_on(api.database().raw_cmd("DROP DATABASE `other_schema`"))
        .ok();
    api.raw_cmd("CREATE DATABASE `other_schema`");

    let sql = r#"
        CREATE TABLE `other_schema`.`User` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `other_schema`.`Post` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            user_id INTEGER,
            FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE CASCADE
        );
    "#;

    api.raw_cmd(sql);

    // Now the migration in the current database.
    let sql = r#"
        CREATE TABLE `User` (
            id VARCHAR(100) PRIMARY KEY
        );

        CREATE TABLE `Post` (
            id VARCHAR(40) PRIMARY KEY,
            user_id VARCHAR(100),
            FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE RESTRICT ON UPDATE RESTRICT
        );
    "#;

    api.raw_cmd(sql);

    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Post",
                },
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "varchar(40)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        40,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "user_id",
                        tpe: ColumnType {
                            full_data_type: "varchar(100)",
                            family: String,
                            arity: Nullable,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        100,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "varchar(100)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        100,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [
                ForeignKey {
                    constrained_table: TableId(
                        0,
                    ),
                    referenced_table: TableId(
                        1,
                    ),
                    constraint_name: Some(
                        "Post_ibfk_1",
                    ),
                    on_delete_action: Restrict,
                    on_update_action: Restrict,
                },
            ],
            foreign_key_columns: [
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: ColumnId(
                        1,
                    ),
                    referenced_column: ColumnId(
                        2,
                    ),
                },
            ],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "user_id",
                    tpe: Normal,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: ColumnId(
                        1,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        2,
                    ),
                    column_id: ColumnId(
                        2,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql))]
fn introspected_default_strings_should_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE `User` (
            favouriteQuote VARCHAR(500) NOT NULL DEFAULT '"That\'s a lot of fish!"\n - Godzilla, 1998'
        )
    "#;

    api.raw_cmd(create_table);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "favouriteQuote",
                        tpe: ColumnType {
                            full_data_type: "varchar(500)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        500,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "\"That's a lot of fish!\"\n - Godzilla, 1998",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql))]
fn escaped_quotes_in_string_defaults_must_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE `string_defaults_test` (
            `regular` VARCHAR(200) NOT NULL DEFAULT 'meow, says the cat',
            `escaped` VARCHAR(200) NOT NULL DEFAULT '\"That\'s a lot of fish!\"\n- Godzilla, 1998'
        );
    "#;
    api.raw_cmd(create_table);

    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "string_defaults_test",
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "regular",
                        tpe: ColumnType {
                            full_data_type: "varchar(200)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        200,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "meow, says the cat",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "escaped",
                        tpe: ColumnType {
                            full_data_type: "varchar(200)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        200,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "\"That's a lot of fish!\"\n- Godzilla, 1998",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql))]
fn escaped_backslashes_in_string_literals_must_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE test (
            `model_name_space` VARCHAR(255) NOT NULL DEFAULT 'xyz\\Datasource\\Model'
        )
    "#;

    api.raw_cmd(create_table);

    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "test",
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "model_name_space",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "xyz\\Datasource\\Model",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql8))]
fn function_expression_defaults_are_described_as_dbgenerated(api: TestApi) {
    let create_table = r#"
        CREATE TABLE game (
            int_col Int DEFAULT (ABS(8) + ABS(8)),
            bigint_col BigInt DEFAULT (ABS(8)),
            float_col Float DEFAULT (ABS(8)),
            decimal_col Decimal DEFAULT (ABS(8)),
            boolean_col TinyInt(1) DEFAULT (IFNULL(1,0)),
            string_col Varchar(8) DEFAULT (LEFT(UUID(), 8)),
            dt_col DateTime DEFAULT current_timestamp(),
            dt_col2 DateTime DEFAULT (SUBDATE(SYSDATE(), 31)),
            binary_col Binary(16) NOT NULL DEFAULT (conv(10,10,2)),
            json_col Json DEFAULT (Trim('{} ')),
            enum_col ENUM('x-small') DEFAULT (Trim('x-small   ')),
            unsupported_col SET('one', 'two') DEFAULT (Trim(' '))
        );
    "#;

    api.raw_cmd(create_table);

    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "game",
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "game_enum_col",
                    values: [
                        "x-small",
                    ],
                },
            ],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(abs(8) + abs(8))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "bigint",
                            family: BigInt,
                            arity: Nullable,
                            native_type: Some(
                                String(
                                    "BigInt",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(abs(8))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "float",
                            family: Float,
                            arity: Nullable,
                            native_type: Some(
                                String(
                                    "Float",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(abs(8))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "decimal_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(10,0)",
                            family: Decimal,
                            arity: Nullable,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            10,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(abs(8))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "boolean_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint(1)",
                            family: Boolean,
                            arity: Nullable,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(ifnull(1,0))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "string_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(8)",
                            family: String,
                            arity: Nullable,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        8,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(left(uuid(),8))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "dt_col",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Nullable,
                            native_type: Some(
                                Object({
                                    "DateTime": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Now,
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "dt_col2",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Nullable,
                            native_type: Some(
                                Object({
                                    "DateTime": Number(
                                        0,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(sysdate() - interval 31 day)",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "binary(16)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Binary": Number(
                                        16,
                                    ),
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(conv(10,10,2))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "json",
                            family: Json,
                            arity: Nullable,
                            native_type: Some(
                                String(
                                    "Json",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(trim(_utf8mb4\\'{} \\'))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "enum_col",
                        tpe: ColumnType {
                            full_data_type: "enum('x-small')",
                            family: Enum(
                                "game_enum_col",
                            ),
                            arity: Nullable,
                            native_type: None,
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(trim(_utf8mb4\\'x-small   \\'))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "unsupported_col",
                        tpe: ColumnType {
                            full_data_type: "set('one','two')",
                            family: String,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: Some(
                            DefaultValue {
                                kind: DbGenerated(
                                    "(trim(_utf8mb4\\' \\'))",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql), exclude(Mysql8))]
fn dangling_foreign_keys_are_filtered_out(api: TestApi) {
    let setup = r#"
    SET FOREIGN_KEY_CHECKS=0;

    CREATE TABLE `platypus` (
        id INTEGER PRIMARY KEY
    );

    CREATE TABLE `dog` (
        id INTEGER PRIMARY KEY,
        bestFriendId INTEGER,

        FOREIGN KEY (`bestFriendId`) REFERENCES `cat`(`id`),
        FOREIGN KEY (`bestFriendId`) REFERENCES `platypus`(`id`) ON DELETE RESTRICT ON UPDATE RESTRICT,
        FOREIGN KEY (`bestFriendId`) REFERENCES `goat`(`id`)
    );

    SET FOREIGN_KEY_CHECKS=1;
    "#;

    api.raw_cmd(setup);
    let result = api.describe();
    let fks: Vec<_> = result
        .walk_foreign_keys()
        .map(|fk| (fk.constraint_name(), fk.referenced_table().name()))
        .collect();

    let expectation = expect![[r#"
        [
            (
                Some(
                    "dog_ibfk_2",
                ),
                "platypus",
            ),
        ]
    "#]];
    expectation.assert_debug_eq(&fks);
}

#[test_connector(tags(Mysql8))]
fn primary_key_length_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE `A` (
            id TEXT NOT NULL,
            CONSTRAINT PRIMARY KEY (id (255))
        );
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    assert_eq!(1, table.primary_key_columns_count());

    let columns = table.primary_key_columns().unwrap().collect::<Vec<_>>();

    assert_eq!("id", columns[0].as_column().name());
    assert_eq!(Some(255), columns[0].length());
}

#[test_connector(tags(Mysql8))]
fn index_length_and_sorting_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY,
            a  TEXT NOT NULL,
            b  TEXT NOT NULL
        );

        CREATE INDEX foo ON `A` (a (10) ASC, b (20) DESC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    assert_eq!(2, table.indexes().len());

    let index = table.indexes().find(|idx| !idx.is_primary_key()).unwrap();
    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Asc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());

    assert_eq!(Some(10), columns[0].length());
    assert_eq!(Some(20), columns[1].length());
}
