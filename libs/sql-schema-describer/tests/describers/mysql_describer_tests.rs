use crate::test_api::*;
use barrel::{types, Migration};
use native_types::{MySqlType, NativeType};
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

#[test_connector(tags(Mysql))]
fn views_can_be_described(api: TestApi) {
    api.raw_cmd(&format!("CREATE TABLE {}.a (a_id int)", api.db_name()));
    api.raw_cmd(&format!("CREATE TABLE {}.b (b_id int)", api.db_name()));

    let create_view = format!(
        r#"
            CREATE VIEW {0}.ab AS
            SELECT a_id
            FROM {0}.a
            UNION ALL
            SELECT b_id
            FROM {0}.b"#,
        api.db_name()
    );

    api.raw_cmd(&create_view);

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

#[test_connector(tags(Mysql), exclude(Mysql8, Mysql56))]
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
    let result = api.describe();
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int(11)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Int.to_json()),
            },

            default: None,
            auto_increment: true,
        },
        Column {
            name: "int_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int(11)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Int.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "smallint(6)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::SmallInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint4_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyint(4)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyInt.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint1_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyint(1)".into(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumint(9)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "bigint(20)".to_string(),
                family: ColumnTypeFamily::BigInt,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::BigInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "decimal_col".to_string(),
            tpe: ColumnType {
                full_data_type: "decimal(10,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Decimal(Some((10, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".to_string(),
            tpe: ColumnType {
                full_data_type: "decimal(10,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Decimal(Some((10, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".to_string(),
            tpe: ColumnType {
                full_data_type: "float".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Float.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".to_string(),
            tpe: ColumnType {
                full_data_type: "double".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Double.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_col".to_string(),
            tpe: ColumnType {
                full_data_type: "date".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Date.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".to_string(),
            tpe: ColumnType {
                full_data_type: "time".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Time(Some(0)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime_col".to_string(),
            tpe: ColumnType {
                full_data_type: "datetime".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::DateTime(Some(0)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_col".to_string(),
            tpe: ColumnType {
                full_data_type: "timestamp".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Timestamp(Some(0)).to_json()),
            },

            default: Some(DefaultValue::now()),
            auto_increment: false,
        },
        Column {
            name: "year_col".to_string(),
            tpe: ColumnType {
                full_data_type: "year(4)".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Year.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".to_string(),
            tpe: ColumnType {
                full_data_type: "char(1)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Char(1).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varchar(255)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::VarChar(255).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                full_data_type: "text".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Text.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinytext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinytext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumtext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumtext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "longtext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "longtext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::LongText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "enum_col".to_string(),
            tpe: ColumnType {
                full_data_type: "enum(\'a\',\'b\')".to_string(),
                family: ColumnTypeFamily::Enum("User_enum_col".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "set_col".to_string(),
            tpe: ColumnType {
                full_data_type: "set(\'a\',\'b\')".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "binary(1)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Binary(1).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varbinary(255)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::VarBinary(255).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "blob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "blob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Blob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "longblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "longblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::LongBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometry_col".to_string(),
            tpe: ColumnType {
                full_data_type: "geometry".to_string(),
                family: ColumnTypeFamily::Unsupported("geometry".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "point_col".to_string(),
            tpe: ColumnType {
                full_data_type: "point".to_string(),
                family: ColumnTypeFamily::Unsupported("point".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "linestring_col".to_string(),
            tpe: ColumnType {
                full_data_type: "linestring".to_string(),
                family: ColumnTypeFamily::Unsupported("linestring".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "polygon_col".to_string(),
            tpe: ColumnType {
                full_data_type: "polygon".to_string(),
                family: ColumnTypeFamily::Unsupported("polygon".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipoint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multipoint".to_string(),
                family: ColumnTypeFamily::Unsupported("multipoint".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multilinestring_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multilinestring".to_string(),
                family: ColumnTypeFamily::Unsupported("multilinestring".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipolygon_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multipolygon".to_string(),
                family: ColumnTypeFamily::Unsupported("multipolygon".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometrycollection_col".to_string(),
            tpe: ColumnType {
                full_data_type: "geometrycollection".to_string(),
                family: ColumnTypeFamily::Unsupported("geometrycollection".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "json_col".to_string(),
            tpe: ColumnType {
                full_data_type: if api.is_mariadb() {
                    "longtext".into()
                } else {
                    "json".to_string()
                },
                family: if api.is_mariadb() {
                    ColumnTypeFamily::String
                } else {
                    ColumnTypeFamily::Json
                },
                arity: ColumnArity::Required,
                native_type: if api.is_mariadb() {
                    Some(MySqlType::LongText.to_json())
                } else {
                    Some(MySqlType::Json.to_json())
                },
            },
            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        Table {
            name: "User".to_string(),
            columns: expected_columns,
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".to_string()],
                sequence: None,
                constraint_name: None,
            }),
            foreign_keys: vec![],
        }
    );
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
    let result = api.describe();
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Int.to_json()),
            },

            default: None,
            auto_increment: true,
        },
        Column {
            name: "int_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Int.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "smallint".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::SmallInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint4_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyint".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyInt.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint1_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyint(1)".to_string(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumint".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "bigint".to_string(),
                family: ColumnTypeFamily::BigInt,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::BigInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "decimal_col".to_string(),
            tpe: ColumnType {
                full_data_type: "decimal(10,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Decimal(Some((10, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".to_string(),
            tpe: ColumnType {
                full_data_type: "decimal(10,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Decimal(Some((10, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".to_string(),
            tpe: ColumnType {
                full_data_type: "float".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Float.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".to_string(),
            tpe: ColumnType {
                full_data_type: "double".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Double.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_col".to_string(),
            tpe: ColumnType {
                full_data_type: "date".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Date.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".to_string(),
            tpe: ColumnType {
                full_data_type: "time".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Time(Some(0)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime_col".to_string(),
            tpe: ColumnType {
                full_data_type: "datetime".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::DateTime(Some(0)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_col".to_string(),
            tpe: ColumnType {
                full_data_type: "timestamp".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Timestamp(Some(0)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "year_col".to_string(),
            tpe: ColumnType {
                full_data_type: "year".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Year.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".to_string(),
            tpe: ColumnType {
                full_data_type: "char(1)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Char(1).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varchar(255)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::VarChar(255).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                full_data_type: "text".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Text.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinytext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinytext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumtext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumtext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "longtext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "longtext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::LongText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "enum_col".to_string(),
            tpe: ColumnType {
                full_data_type: "enum(\'a\',\'b\')".to_string(),
                family: ColumnTypeFamily::Enum("User_enum_col".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "set_col".to_string(),
            tpe: ColumnType {
                full_data_type: "set(\'a\',\'b\')".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "binary(1)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Binary(1).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varbinary(255)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::VarBinary(255).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "blob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "blob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Blob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::TinyBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "mediumblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::MediumBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "longblob_col".to_string(),
            tpe: ColumnType {
                full_data_type: "longblob".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::LongBlob.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometry_col".to_string(),
            tpe: ColumnType {
                full_data_type: "geometry".to_string(),
                family: ColumnTypeFamily::Unsupported("geometry".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "point_col".to_string(),
            tpe: ColumnType {
                full_data_type: "point".to_string(),
                family: ColumnTypeFamily::Unsupported("point".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "linestring_col".to_string(),
            tpe: ColumnType {
                full_data_type: "linestring".to_string(),
                family: ColumnTypeFamily::Unsupported("linestring".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "polygon_col".to_string(),
            tpe: ColumnType {
                full_data_type: "polygon".to_string(),
                family: ColumnTypeFamily::Unsupported("polygon".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipoint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multipoint".to_string(),
                family: ColumnTypeFamily::Unsupported("multipoint".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multilinestring_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multilinestring".to_string(),
                family: ColumnTypeFamily::Unsupported("multilinestring".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipolygon_col".to_string(),
            tpe: ColumnType {
                full_data_type: "multipolygon".to_string(),
                family: ColumnTypeFamily::Unsupported("multipolygon".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometrycollection_col".to_string(),
            tpe: ColumnType {
                full_data_type: "geomcollection".to_string(),
                family: ColumnTypeFamily::Unsupported("geomcollection".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "json_col".to_string(),
            tpe: ColumnType {
                full_data_type: "json".to_string(),
                family: ColumnTypeFamily::Json,
                arity: ColumnArity::Required,
                native_type: Some(MySqlType::Json.to_json()),
            },
            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        Table {
            name: "User".to_string(),
            columns: expected_columns,
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".to_string()],
                sequence: None,
                constraint_name: None,
            }),
            foreign_keys: vec![],
        }
    );
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
    let table = result.get_table("Employee").expect("couldn't get Employee table");

    assert_eq!(
        table.indices,
        &[Index {
            name: "age_and_name_index".into(),
            columns: vec!["name".to_owned(), "age".to_owned()],
            tpe: IndexType::Unique,
        }]
    );
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
    let mut other_migration = Migration::new().schema("other_schema");

    other_migration.create_table("User", |t| {
        t.add_column("id", types::primary());
    });
    other_migration.create_table("Post", |t| {
        t.add_column("id", types::primary());
        t.inject_custom("user_id INTEGER, FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE CASCADE");
    });

    let full_sql = other_migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);

    let schema = api
        .block_on(api.describer().describe(&"other_schema"))
        .expect("describing");
    let table = schema.table_bang("Post");

    let fks = &table.foreign_keys;

    assert_eq!(
        fks,
        &[ForeignKey {
            constraint_name: Some("Post_ibfk_1".into()),
            columns: vec!["user_id".into()],
            referenced_table: "User".into(),
            referenced_columns: vec!["id".into()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
        }]
    );

    // Now the migration in the current database.

    let mut migration = Migration::new().schema(api.db_name());

    migration.create_table("User", |t| {
        t.add_column("id", types::primary());
    });

    migration.create_table("Post", |t| {
        t.add_column("id", types::primary());
        t.inject_custom("user_id INTEGER, FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE RESTRICT");
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    api.raw_cmd(&full_sql);
    let schema = api.describe();
    let table = schema.table_bang("Post");

    let fks = &table.foreign_keys;

    assert_eq!(
        fks,
        &[ForeignKey {
            constraint_name: Some("Post_ibfk_1".into()),
            columns: vec!["user_id".into()],
            referenced_table: "User".into(),
            referenced_columns: vec!["id".into()],
            on_delete_action: ForeignKeyAction::Restrict,
            on_update_action: ForeignKeyAction::NoAction,
        }]
    );
}

#[test_connector(tags(Mysql))]
fn mysql_introspected_default_strings_should_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE `mysql_introspected_default_strings_should_be_unescaped`.`User` (
            id INTEGER PRIMARY KEY,
            favouriteQuote VARCHAR(500) NOT NULL DEFAULT '"That\'s a lot of fish!"\n - Godzilla, 1998'
        )
    "#;

    api.raw_cmd(&create_table);
    let schema = api.describe();

    let expected_default = prisma_value::PrismaValue::String(
        r#""That's a lot of fish!"
 - Godzilla, 1998"#
            .into(),
    );

    let table = schema.table_bang("User");
    let column = table.column_bang("favouriteQuote");

    let actual_default = column.default.as_ref().unwrap().as_value().unwrap();

    assert_eq!(actual_default, &expected_default);
}

#[test_connector(tags(Mysql))]
fn escaped_quotes_in_string_defaults_must_be_unescaped(api: TestApi) {
    let create_table = format!(
        r#"
            CREATE TABLE `{0}`.`string_defaults_test` (
                `id` INTEGER PRIMARY KEY,
                `regular` VARCHAR(200) NOT NULL DEFAULT 'meow, says the cat',
                `escaped` VARCHAR(200) NOT NULL DEFAULT '\"That\'s a lot of fish!\"\n- Godzilla, 1998'
            );
        "#,
        api.schema_name()
    );

    api.raw_cmd(&create_table);

    let schema = api.describe();

    let table = schema.table_bang("string_defaults_test");

    let regular_column_default = table
        .column_bang("regular")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(regular_column_default, "meow, says the cat");

    let escaped_column_default = table
        .column_bang("escaped")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(
        escaped_column_default,
        r#""That's a lot of fish!"
- Godzilla, 1998"#
    );
}

#[test_connector(tags(Mysql))]
fn escaped_backslashes_in_string_literals_must_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE test (
            `model_name_space` VARCHAR(255) NOT NULL DEFAULT 'xyz\\Datasource\\Model'
        )
    "#;

    api.raw_cmd(&create_table);

    let schema = api.describe();

    let table = schema.table_bang("test");

    let default = table
        .column_bang("model_name_space")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(default, "xyz\\Datasource\\Model");
}

#[test_connector(tags(Mysql8, Mariadb))]
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

    api.raw_cmd(&create_table);

    let schema = api.describe();

    let table = schema.table_bang("game");

    let default = |name| table.column_bang(name).default.as_ref().unwrap();

    assert_eq!(default("int_col"), &DefaultValue::db_generated("(abs(8) + abs(8))"));
    assert_eq!(default("bigint_col"), &DefaultValue::db_generated("(abs(8))"));
    assert_eq!(default("float_col"), &DefaultValue::db_generated("(abs(8))"));
    assert_eq!(default("decimal_col"), &DefaultValue::db_generated("(abs(8))"));
    assert_eq!(default("boolean_col"), &DefaultValue::db_generated("(ifnull(1,0))"));
    assert_eq!(default("string_col"), &DefaultValue::db_generated("(left(uuid(),8))"));
    assert_eq!(default("dt_col"), &DefaultValue::now());
    assert_eq!(
        default("dt_col2"),
        &DefaultValue::db_generated("(sysdate() - interval 31 day)")
    );
    assert_eq!(default("binary_col"), &DefaultValue::db_generated("(conv(10,10,2))"));
    //todo strings are returned differently on mysql8
    // assert_eq!(default("json_col"), &DefaultValue::db_generated("(trim(\'{} \'))"));
    // assert_eq!(
    //     default("enum_col"),
    //     &DefaultValue::db_generated("(trim(\'x-small   \'))")
    // );
    // assert_eq!(default("unsupported_col"), &DefaultValue::db_generated("(trim(\' \'))"));
}

#[test_connector(tags(Mysql))]
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
        FOREIGN KEY (`bestFriendId`) REFERENCES `platypus`(`id`),
        FOREIGN KEY (`bestFriendId`) REFERENCES `goat`(`id`)
    );

    SET FOREIGN_KEY_CHECKS=1;
    "#;

    api.raw_cmd(setup);

    let schema = api.describe();
    let table = schema.table_bang("dog");

    assert!(
        matches!(table.foreign_keys.as_slice(), [fk] if fk.referenced_table == "platypus"),
        "{:#?}",
        table.foreign_keys
    );
}
