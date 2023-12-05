use crate::test_api::*;
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
    let sql = r#"
        CREATE TABLE `User` (
        id INTEGER AUTO_INCREMENT PRIMARY KEY,
        `int_col` int,
        `smallint_col` smallint,
        `tinyint4_col` tinyint(4),
        `tinyint1_col` tinyint(1),
        `mediumint_col` mediumint,
        `bigint_col` bigint,
        `decimal_col` decimal,
        `numeric_col` numeric,
        `float_col` float,
        `double_col` double,
        `date_col` date,
        `time_col` time,
        `datetime_col` datetime,
        `timestamp_col` timestamp,
        `year_col` year,
        `char_col` char,
        `varchar_col` varchar(255),
        `text_col` text,
        `tinytext_col` tinytext,
        `mediumtext_col` mediumtext,
        `longtext_col` longtext,
        `enum_col` enum('a', 'b'),
        `set_col` set('a', 'b'),
        `binary_col` binary,
        `varbinary_col` varbinary(255),
        `blob_col` blob,
        `tinyblob_col` tinyblob,
        `mediumblob_col` mediumblob,
        `longblob_col` longblob,
        `geometry_col` geometry,
        `point_col` point,
        `linestring_col` linestring,
        `polygon_col` polygon,
        `multipoint_col` multipoint,
        `multilinestring_col` multilinestring,
        `multipolygon_col` multipolygon,
        `geometrycollection_col` geometrycollection,
        `json_col` json
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
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    description: None,
                },
            ],
            enum_variants: [
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "a",
                },
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "b",
                },
            ],
            table_columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "int(11)",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                EnumId(
                                    0,
                                ),
                            ),
                            arity: Nullable,
                            native_type: None,
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: None,
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        14,
                    ),
                    DefaultValue {
                        kind: Now,
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
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
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mariadb))]
fn all_mariadb_column_types_must_work(api: TestApi) {
    let sql = r#"
        CREATE TABLE `User` (
        primary_col INTEGER AUTO_INCREMENT PRIMARY KEY,
        `int_col` int NOT NULL,
        `smallint_col` smallint NOT NULL,
        `tinyint4_col` tinyint(4) NOT NULL,
        `tinyint1_col` tinyint(1) NOT NULL,
        `mediumint_col` mediumint NOT NULL,
        `bigint_col` bigint NOT NULL,
        `decimal_col` decimal NOT NULL,
        `numeric_col` numeric NOT NULL,
        `float_col` float NOT NULL,
        `double_col` double NOT NULL,
        `date_col` date NOT NULL,
        `time_col` time NOT NULL,
        `datetime_col` datetime NOT NULL,
        `timestamp_col` timestamp NOT NULL,
        `year_col` year NOT NULL,
        `char_col` char NOT NULL,
        `varchar_col` varchar(255) NOT NULL,
        `text_col` text NOT NULL,
        `tinytext_col` tinytext NOT NULL,
        `mediumtext_col` mediumtext NOT NULL,
        `longtext_col` longtext NOT NULL,
        `enum_col` enum('a', 'b') NOT NULL,
        `set_col` set('a', 'b') NOT NULL,
        `binary_col` binary NOT NULL,
        `varbinary_col` varbinary(255) NOT NULL,
        `blob_col` blob NOT NULL,
        `tinyblob_col` tinyblob NOT NULL,
        `mediumblob_col` mediumblob NOT NULL,
        `longblob_col` longblob NOT NULL,
        `geometry_col` geometry NOT NULL,
        `point_col` point NOT NULL,
        `linestring_col` linestring NOT NULL,
        `polygon_col` polygon NOT NULL,
        `multipoint_col` multipoint NOT NULL,
        `multilinestring_col` multilinestring NOT NULL,
        `multipolygon_col` multipolygon NOT NULL,
        `geometrycollection_col` geometrycollection NOT NULL,
        `json_col` json NOT NULL
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
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    description: None,
                },
            ],
            enum_variants: [
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "a",
                },
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "b",
                },
            ],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                EnumId(
                                    0,
                                ),
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [],
            view_default_values: [],
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
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql8))]
fn all_mysql_8_column_types_must_work(api: TestApi) {
    let sql = r#"
        CREATE TABLE `User` (
        primary_col INTEGER AUTO_INCREMENT PRIMARY KEY,
        `int_col` int NOT NULL,
        `smallint_col` smallint NOT NULL,
        `tinyint4_col` tinyint(4) NOT NULL,
        `tinyint1_col` tinyint(1) NOT NULL,
        `mediumint_col` mediumint NOT NULL,
        `bigint_col` bigint NOT NULL,
        `decimal_col` decimal NOT NULL,
        `numeric_col` numeric NOT NULL,
        `float_col` float NOT NULL,
        `double_col` double NOT NULL,
        `date_col` date NOT NULL,
        `time_col` time NOT NULL,
        `datetime_col` datetime NOT NULL,
        `timestamp_col` timestamp NOT NULL,
        `year_col` year NOT NULL,
        `char_col` char NOT NULL,
        `varchar_col` varchar(255) NOT NULL,
        `text_col` text NOT NULL,
        `tinytext_col` tinytext NOT NULL,
        `mediumtext_col` mediumtext NOT NULL,
        `longtext_col` longtext NOT NULL,
        `enum_col` enum('a', 'b') NOT NULL,
        `set_col` set('a', 'b') NOT NULL,
        `binary_col` binary NOT NULL,
        `varbinary_col` varbinary(255) NOT NULL,
        `blob_col` blob NOT NULL,
        `tinyblob_col` tinyblob NOT NULL,
        `mediumblob_col` mediumblob NOT NULL,
        `longblob_col` longblob NOT NULL,
        `geometry_col` geometry srid 4326 NOT NULL,
        `point_col` point srid 4326 NOT NULL,
        `linestring_col` linestring srid 4326 NOT NULL,
        `polygon_col` polygon srid 4326 NOT NULL,
        `multipoint_col` multipoint srid 4326 NOT NULL,
        `multilinestring_col` multilinestring srid 4326 NOT NULL,
        `multipolygon_col` multipolygon srid 4326 NOT NULL,
        `geometrycollection_col` geometrycollection srid 4326 NOT NULL,
        `json_col` json NOT NULL
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
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_enum_col",
                    description: None,
                },
            ],
            enum_variants: [
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "a",
                },
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "b",
                },
            ],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                EnumId(
                                    0,
                                ),
                            ),
                            arity: Required,
                            native_type: None,
                        },
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                            family: Geometry,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [],
            view_default_values: [],
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
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
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

#[test_connector(tags(Mysql))]
fn mysql_join_table_unique_indexes_must_be_inferred(api: TestApi) {
    let sql = r#"
        CREATE TABLE `Cat` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            name TEXT
        );

        CREATE TABLE `Human` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            name TEXT
        );

        CREATE TABLE `CatToHuman` (
            cat INTEGER REFERENCES `Cat`(id),
            human INTEGER REFERENCES `Human`(id),
            relationship TEXT
        );

        CREATE UNIQUE INDEX cat_and_human_index ON `CatToHuman`(cat, human);
    "#;
    api.raw_cmd(sql);

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
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
            table_default_values: [],
            view_default_values: [],
            foreign_key_columns: [
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: TableColumnId(
                        1,
                    ),
                    referenced_column: TableColumnId(
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
                    column_id: TableColumnId(
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
                    column_id: TableColumnId(
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
                    column_id: TableColumnId(
                        2,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
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
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        0,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "\"That's a lot of fish!\"\n - Godzilla, 1998",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            check_constraints: [],
            views: [],
            view_columns: [],
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
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        0,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "meow, says the cat",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        1,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "\"That's a lot of fish!\"\n- Godzilla, 1998",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mysql))]
fn escaped_backslashes_in_string_literals_must_be_unescaped(api: TestApi) {
    let create_table = r"
        CREATE TABLE test (
            `model_name_space` VARCHAR(255) NOT NULL DEFAULT 'xyz\\Datasource\\Model'
        )
    ";

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
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        0,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "xyz\\Datasource\\Model",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            check_constraints: [],
            views: [],
            view_columns: [],
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
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "game_enum_col",
                    description: None,
                },
            ],
            enum_variants: [
                EnumVariant {
                    enum_id: EnumId(
                        0,
                    ),
                    variant_name: "x-small",
                },
            ],
            table_columns: [
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
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
                                EnumId(
                                    0,
                                ),
                            ),
                            arity: Nullable,
                            native_type: None,
                        },
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        0,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(abs(8) + abs(8))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        1,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(abs(8))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        2,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(abs(8))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        3,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(abs(8))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        4,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(ifnull(1,0))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        5,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(left(uuid(),8))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        6,
                    ),
                    DefaultValue {
                        kind: Now,
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        7,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(sysdate() - interval 31 day)",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        8,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(conv(10,10,2))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        9,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(trim(_utf8mb4\\'{} \\'))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        10,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(trim(_utf8mb4\\'x-small   \\'))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        11,
                    ),
                    DefaultValue {
                        kind: DbGenerated(
                            Some(
                                "(trim(_utf8mb4\\' \\'))",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            check_constraints: [],
            views: [],
            view_columns: [],
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
