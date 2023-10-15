use super::*;
use crate::{
    connector::Queryable,
    error::{DatabaseConstraint, ErrorKind, Name},
};
use quaint_test_macros::test_each_connector;

#[test_each_connector]
async fn table_does_not_exist(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::from_table("not_there");

    let err = api.conn().select(select).await.unwrap_err();

    match err.kind() {
        ErrorKind::TableDoesNotExist { table } => {
            assert_eq!(&Name::available("not_there"), table);
        }
        e => panic!("Expected error TableDoesNotExist, got {:?}", e),
    }

    Ok(())
}

#[test_each_connector(tags("mssql"))]
async fn database_already_exists(api: &mut dyn TestApi) -> crate::Result<()> {
    let query = "CREATE DATABASE master";

    let err = api.conn().raw_cmd(query).await.unwrap_err();

    match err.kind() {
        ErrorKind::DatabaseAlreadyExists { db_name } => {
            assert_eq!(&Name::available("master"), db_name);
        }
        e => panic!("Expected error DatabaseAlreadyExists, got {:?}", e),
    }

    Ok(())
}

#[test_each_connector]
async fn column_does_not_exist_on_write(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 int").await?;

    let insert = Insert::single_into(&table).value("id1", 1).value("does_not_exist", 2);
    let res = api.conn().insert(insert.clone().into()).await;

    assert!(res.is_err());

    let err = res.unwrap_err();

    match err.kind() {
        ErrorKind::ColumnNotFound { column } => {
            assert_eq!(&Name::available("does_not_exist"), column);
        }
        e => panic!("Expected error ColumnNotFound, got {:?}", e),
    }

    Ok(())
}

#[test_each_connector]
async fn column_does_not_exist_on_read(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 int").await?;

    let insert = Insert::single_into(&table).value("id1", 1);
    api.conn().insert(insert.clone().into()).await?;

    let select = format!("Select does_not_exist from {table}");
    let res = api.conn().query_raw(&select, &[]).await;

    assert!(res.is_err());

    let err = res.unwrap_err();

    match err.kind() {
        ErrorKind::ColumnNotFound { column } => {
            assert_eq!(&Name::available("does_not_exist"), column);
        }
        e => panic!("Expected error ColumnNotFound, got {:?}", e),
    }

    Ok(())
}

#[test_each_connector]
async fn unique_constraint_violation(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 int, id2 int").await?;
    let index = api.create_index(&table, "id1, id2").await?;

    let insert = Insert::single_into(&table).value("id1", 1).value("id2", 2);
    api.conn().insert(insert.clone().into()).await?;

    let res = api.conn().insert(insert.clone().into()).await;

    assert!(res.is_err());

    let err = res.unwrap_err();

    match &err.kind() {
        ErrorKind::UniqueConstraintViolation { constraint } => match constraint {
            DatabaseConstraint::Index(idx) => assert_eq!(&index, idx),
            DatabaseConstraint::Fields(fields) => {
                let fields = fields.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                assert_eq!(vec!["id1", "id2"], fields)
            }
            DatabaseConstraint::ForeignKey => panic!("Expecting index or field constraints."),
            DatabaseConstraint::CannotParse => panic!("Couldn't parse the error message."),
        },
        _ => panic!("{}", err),
    }

    Ok(())
}

#[test_each_connector]
async fn null_constraint_violation(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 int not null, id2 int not null").await?;

    let res = api.conn().insert(Insert::single_into(&table).into()).await;
    let err = res.unwrap_err();

    match err.kind() {
        ErrorKind::NullConstraintViolation { constraint } => {
            assert_eq!(&DatabaseConstraint::fields(Some("id1")), constraint)
        }
        _ => panic!("{}", err),
    }

    let insert = Insert::single_into(&table).value("id1", 50).value("id2", 55);
    api.conn().insert(insert.into()).await?;

    let update = Update::table(&table).set("id2", ValueType::Int64(None));
    let res = api.conn().update(update).await;

    assert!(res.is_err());

    let err = res.unwrap_err();

    match err.kind() {
        ErrorKind::NullConstraintViolation { constraint } => {
            assert_eq!(&DatabaseConstraint::fields(Some("id2")), constraint);
        }
        _ => panic!("{}", err),
    }

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn int_unsigned_negative_value_out_of_range(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int4 auto_increment primary key, big int4 unsigned")
        .await?;

    // Negative value
    {
        let insert = Insert::multi_into(&table, ["big"]).values((-22,));
        let result = api.conn().insert(insert.into()).await;

        assert!(matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { .. }));
    }

    // Value too big
    {
        let insert = Insert::multi_into(&table, ["big"]).values((std::i64::MAX,));
        let result = api.conn().insert(insert.into()).await;

        assert!(matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { .. }));
    }

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn bigint_unsigned_positive_value_out_of_range(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int4 auto_increment primary key, big bigint unsigned")
        .await?;

    let insert = format!(r#"INSERT INTO `{table}` (`big`) VALUES (18446744073709551615)"#);
    api.conn().execute_raw(&insert, &[]).await.unwrap();
    let result = api.conn().select(Select::from_table(&table)).await;

    assert!(
        matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { message } if message == "Unsigned integers larger than 9_223_372_036_854_775_807 are currently not handled.")
    );

    Ok(())
}

#[test_each_connector(tags("mysql", "mssql", "postgresql"))]
async fn length_mismatch(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("value varchar(3)").await?;
    let insert = Insert::single_into(&table).value("value", "fooo");

    let result = api.conn().insert(insert.into()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::LengthMismatch { .. }));

    Ok(())
}

#[test_each_connector(tags("postgresql", "sqlite"))]
async fn foreign_key_constraint_violation(api: &mut dyn TestApi) -> crate::Result<()> {
    let parent = api.create_temp_table("id smallint not null primary key").await?;
    let foreign_key = api.foreign_key(&parent, "id", "parent_id");
    let child = api
        .create_temp_table(&format!("parent_id smallint not null, {}", &foreign_key))
        .await?;

    let insert = Insert::single_into(&child).value("parent_id", 10);
    let result = api.conn().insert(insert.into()).await;

    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::ForeignKeyConstraintViolation { .. }));

    Ok(())
}

/// SQL Server and MySQL do not allow foreign keys in temporary tables, so
/// we'll do them separately.
#[test_each_connector(tags("mssql", "mysql"))]
async fn ms_my_foreign_key_constraint_violation(api: &mut dyn TestApi) -> crate::Result<()> {
    let parent_table = api.get_name();
    let child_table = api.get_name();
    let constraint = api.get_name();

    let create_table = format!("CREATE TABLE {parent_table} (id smallint not null primary key)");
    api.conn().raw_cmd(&create_table).await?;

    let create_table = format!(
        r#"
        CREATE TABLE {} (
            parent_id smallint not null,
            CONSTRAINT {} FOREIGN KEY (parent_id) REFERENCES {}(id))
        "#,
        &child_table, &constraint, &parent_table
    );

    api.conn().raw_cmd(&create_table).await?;

    let insert = Insert::single_into(&child_table).value("parent_id", 10);
    let result = api.conn().insert(insert.into()).await;

    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::ForeignKeyConstraintViolation { .. }));

    api.conn().raw_cmd(&format!("DROP TABLE {}", &child_table)).await?;
    api.conn().raw_cmd(&format!("DROP TABLE {}", &parent_table)).await?;

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn garbage_datetime_values(api: &mut dyn TestApi) -> crate::Result<()> {
    api.conn()
        .raw_cmd("set @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO'")
        .await?;

    let table = api
        .create_temp_table("data datetime not null default '0000-00-00 00:00:00'")
        .await?;

    let insert = format!("INSERT INTO {table} () VALUES ()");
    api.conn().raw_cmd(&insert).await?;

    let res = api.conn().select(Select::from_table(&table)).await;
    assert!(res.is_err());

    let err = res.unwrap_err();

    match err.kind() {
        ErrorKind::ValueOutOfRange { message } => {
            let expected_message =
                "The column `data` contained an invalid datetime value with either day or month set to zero."
                    .to_string();

            assert_eq!(&expected_message, message);
        }
        e => panic!("Expected error ColumnNotFound, got {:?}", e),
    }

    Ok(())
}

#[test_each_connector]
async fn should_pick_up_partially_failed_raw_cmd_scripts(api: &mut dyn TestApi) -> crate::Result<()> {
    let conn = api.conn();

    let result = conn.raw_cmd("SELECT YOLO; SELECT 1;").await;

    assert!(result.is_err());

    let result = conn.raw_cmd("SELECT 1; SELECT NULL; SELECT YOLO; SELECT 2;").await;

    assert!(result.is_err());

    if api.conn().connection_info().sql_family().is_mysql() {
        let error_message = result.unwrap_err().to_string();
        assert_eq!(error_message, "Error accessing result set, column not found: YOLO");
    }

    Ok(())
}

#[test_each_connector]
async fn should_execute_multi_statement_queries_with_raw_cmd(api: &mut dyn TestApi) -> crate::Result<()> {
    let (table_name_1, create_table_1) = api.render_create_table("testtable", "id INTEGER PRIMARY KEY");
    let (table_name_2, create_table_2) = api.render_create_table("testtable2", "id INTEGER PRIMARY KEY");
    let conn = api.conn();

    let query = format!(
        r#"
        {create_table_1};
        {create_table_2};
        INSERT INTO {table_name_1} (id) VALUES (51);
        INSERT INTO {table_name_2} (id) VALUES (52);
        "#,
    );

    conn.raw_cmd(&query).await.unwrap();

    let results = conn
        .query(Select::from_table(table_name_1).column("id").into())
        .await
        .unwrap();

    let results: Vec<i64> = results
        .into_iter()
        .map(|row| row.get("id").unwrap().as_integer().unwrap())
        .collect();

    assert_eq!(results, &[51]);

    let results = conn
        .query(Select::from_table(table_name_2).column("id").into())
        .await
        .unwrap();

    let results: Vec<i64> = results
        .into_iter()
        .map(|row| row.get("id").unwrap().as_integer().unwrap())
        .collect();

    assert_eq!(results, &[52]);

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn uuid_length_error(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("value uuid").await?;
    let insert = Insert::single_into(&table).value("value", "fooo");

    let result = api.conn().insert(insert.into()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::UUIDError { .. }));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn unsupported_column_type(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("point point, points point[]").await?;
    api.conn()
        .query_raw(
            &format!(
                r#"INSERT INTO {} ("point", "points") VALUES (Point(1,2), '{{"(1, 2)", "(2, 3)"}}')"#,
                &table
            ),
            &[],
        )
        .await?;

    // Scalar
    let result = api
        .conn()
        .query(Select::from_table(&table).column("point").into())
        .await;
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind(),
        ErrorKind::UnsupportedColumnType { column_type } if column_type.as_str() == "point"
    ));

    // Scalar list
    let result = api
        .conn()
        .query(Select::from_table(&table).column("points").into())
        .await;
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind(),
        ErrorKind::UnsupportedColumnType { column_type } if column_type.as_str() == "_point"
    ));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn array_into_scalar_should_fail(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("string text").await?;
    let insert = Insert::single_into(&table).value("string", Value::array(vec!["abc", "def"]));
    let result = api.conn().insert(insert.into()).await;

    let err = result.unwrap_err();

    assert!(err.to_string().contains("Couldn't serialize value"));
    assert!(err.to_string().contains("Value is a list but `text` is not."));

    Ok(())
}

// SQLite errors on anything other than serializable.
#[test_each_connector(tags("sqlite"))]
async fn sqlite_isolation_error(api: &mut dyn TestApi) -> crate::Result<()> {
    let res = api
        .conn()
        .start_transaction(Some(IsolationLevel::ReadUncommitted))
        .await;

    let err = res.err().expect("SQLite must fail on isolation != SERIALIZABLE");
    assert_eq!(err.to_string(), "Invalid isolation level: READ UNCOMMITTED");

    Ok(())
}

// Postgres and MySQL error on Snapshot.
#[test_each_connector(tags("postgresql", "mysql"))]
async fn snapshot_isolation_error(api: &mut dyn TestApi) -> crate::Result<()> {
    let res = api.conn().start_transaction(Some(IsolationLevel::Snapshot)).await;

    let err = res.err().expect("Postgres/MySQL must fail on isolation SNAPSHOT");
    assert_eq!(err.to_string(), "Invalid isolation level: SNAPSHOT");

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn concurrent_transaction_conflict(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.get_name();
    let create_table = format!("CREATE TABLE {table} (id int not null primary key, count int)");
    api.conn().raw_cmd(&create_table).await?;
    api.conn()
        .insert(Insert::single_into(&table).value("id", 1).value("count", 1).into())
        .await?;

    let conn1 = api.create_additional_connection().await?;
    let conn2 = api.create_additional_connection().await?;

    let mut tx1 = conn1.start_transaction(Some(IsolationLevel::Serializable)).await?;
    let tx2 = conn2.start_transaction(Some(IsolationLevel::Serializable)).await?;

    tx1.query(Select::from_table(&table).into()).await?;
    tx2.query(Select::from_table(&table).into()).await?;

    tx1.update(Update::table(&table).set("count", 2)).await?;
    tx1.commit().await?;

    let res = tx2.update(Update::table(&table).set("count", 3));

    let err = res.await.expect_err("Conflicting transaction must fail");

    assert!(matches!(err.kind(), ErrorKind::TransactionWriteConflict));

    Ok(())
}
