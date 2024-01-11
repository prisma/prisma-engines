mod error;

use super::test_api::*;
#[cfg(any(feature = "postgresql", feature = "mysql"))]
use crate::ast::JsonPath;
use crate::{
    connector::{IsolationLevel, Queryable, TransactionCapable},
    error::ErrorKind,
    prelude::*,
};
use quaint_test_macros::test_each_connector;
use quaint_test_setup::Tags;

#[test_each_connector]
async fn single_value(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value("foo");
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::text("foo"), res[0]);

    Ok(())
}

#[test_each_connector]
async fn aliased_value(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(val!("foo").alias("bar"));
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::text("foo"), res["bar"]);

    Ok(())
}

#[test_each_connector]
async fn aliased_null(api: &mut dyn TestApi) -> crate::Result<()> {
    let query = Select::default().value(val!(Value::null_int64()).alias("test"));

    let res = api.conn().select(query).await?;
    let row = res.get(0).unwrap();

    // No results expected.
    assert!(row["test"].is_null());

    Ok(())
}

#[test_each_connector]
async fn select_star_from(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, value int").await?;

    let insert = Insert::single_into(&table).value("value", 3).value("id", 4);
    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::int32(4), row["id"]);
    assert_eq!(Value::int32(3), row["value"]);

    Ok(())
}

#[test_each_connector]
async fn transactions(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("value int").await?;

    let mut tx = api.conn().start_transaction(None).await?;
    let insert = Insert::single_into(&table).value("value", 10);

    let rows_affected = tx.execute(insert.into()).await?;
    assert_eq!(1, rows_affected);

    let select = Select::from_table(&table).column("value");
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::int32(10), res[0]);

    // Check that nested transactions are also rolled back, even at multiple levels deep
    let mut tx_inner = api.conn().start_transaction(None).await?;
    let inner_insert1 = Insert::single_into(&table).value("value", 20);
    let inner_rows_affected1 = tx.execute(inner_insert1.into()).await?;
    assert_eq!(1, inner_rows_affected1);

    let mut tx_inner2 = api.conn().start_transaction(None).await?;
    let inner_insert2 = Insert::single_into(&table).value("value", 20);
    let inner_rows_affected2 = tx.execute(inner_insert2.into()).await?;
    assert_eq!(1, inner_rows_affected2);
    tx_inner2.commit().await?;

    tx_inner.commit().await?;

    tx.rollback().await?;

    let select = Select::from_table(&table).column("value");
    let res = api.conn().select(select).await?;

    assert_eq!(0, res.len());

    Ok(())
}

#[test_each_connector(tags("mssql", "postgresql", "mysql"))]
async fn transactions_with_isolation_works(api: &mut dyn TestApi) -> crate::Result<()> {
    // This test only tests that the SET isolation level statements are accepted.
    api.conn()
        .start_transaction(Some(IsolationLevel::ReadUncommitted))
        .await?
        .commit()
        .await?;

    api.conn()
        .start_transaction(Some(IsolationLevel::ReadCommitted))
        .await?
        .commit()
        .await?;

    api.conn()
        .start_transaction(Some(IsolationLevel::RepeatableRead))
        .await?
        .commit()
        .await?;

    api.conn()
        .start_transaction(Some(IsolationLevel::Serializable))
        .await?
        .commit()
        .await?;

    Ok(())
}

// SQLite only supports serializable.
#[test_each_connector(tags("sqlite"))]
async fn sqlite_serializable_tx(api: &mut dyn TestApi) -> crate::Result<()> {
    api.conn()
        .start_transaction(Some(IsolationLevel::Serializable))
        .await?
        .commit()
        .await?;

    Ok(())
}

// Only SQL Server supports snapshot.
#[test_each_connector(tags("mssql"))]
async fn mssql_snapshot_tx(api: &mut dyn TestApi) -> crate::Result<()> {
    api.conn()
        .start_transaction(Some(IsolationLevel::Snapshot))
        .await?
        .commit()
        .await?;

    Ok(())
}

#[test_each_connector]
async fn in_values_singular(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("id".in_selection(vec![1, 3]));

    let res = api.conn().select(query).await?;
    assert_eq!(2, res.len());

    let row1 = res.get(0).unwrap();
    assert_eq!(Some(1), row1["id"].as_i32());
    assert_eq!(Some(2), row1["id2"].as_i32());

    let row2 = res.get(1).unwrap();
    assert_eq!(Some(3), row2["id"].as_i32());
    assert_eq!(Some(4), row2["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn not_in_values_singular(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("id".not_in_selection(vec![1, 3]));

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row1 = res.get(0).unwrap();
    assert_eq!(Some(5), row1["id"].as_i32());
    assert_eq!(Some(6), row1["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn in_values_tuple(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query =
        Select::from_table(table).so_that(Row::from((col!("id"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

    let res = api.conn().select(query).await?;
    assert_eq!(2, res.len());

    let row1 = res.get(0).unwrap();
    assert_eq!(Some(1), row1["id"].as_i32());
    assert_eq!(Some(2), row1["id2"].as_i32());

    let row2 = res.get(1).unwrap();
    assert_eq!(Some(3), row2["id"].as_i32());
    assert_eq!(Some(4), row2["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn not_in_values_tuple(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table)
        .so_that(Row::from((col!("id"), col!("id2"))).not_in_selection(values!((1, 2), (3, 4))));

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(5), row["id"].as_i32());
    assert_eq!(Some(6), row["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn order_by_ascend(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![3, 4])
        .values(vec![1, 2])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id2".ascend());

    let res = api.conn().select(query).await?;
    assert_eq!(3, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(2), row["id2"].as_i32());

    let row = res.get(1).unwrap();
    assert_eq!(Some(3), row["id"].as_i32());
    assert_eq!(Some(4), row["id2"].as_i32());

    let row = res.get(2).unwrap();
    assert_eq!(Some(5), row["id"].as_i32());
    assert_eq!(Some(6), row["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn order_by_descend(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![3, 4])
        .values(vec![1, 2])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id2".descend());

    let res = api.conn().select(query).await?;
    assert_eq!(3, res.len());
    let row = res.get(0).unwrap();
    assert_eq!(Some(5), row["id"].as_i32());
    assert_eq!(Some(6), row["id2"].as_i32());

    let row = res.get(1).unwrap();
    assert_eq!(Some(3), row["id"].as_i32());
    assert_eq!(Some(4), row["id2"].as_i32());

    let row = res.get(2).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(2), row["id2"].as_i32());

    Ok(())
}

#[test_each_connector]
async fn where_equals(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("name".equals("Naukio"));
    let res = api.conn().select(query).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn where_like(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("name".like("%auk%"));
    let res = api.conn().select(query).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn where_not_like(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("name".not_like("%auk%"));
    let res = api.conn().select(query).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn inner_join(api: &mut dyn TestApi) -> crate::Result<()> {
    let table1 = api.create_temp_table("id int, name varchar(255)").await?;
    let table2 = api.create_temp_table("t1_id int, is_cat int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"])
        .values(vec![Value::int32(1), Value::int32(1)])
        .values(vec![Value::int32(2), Value::int32(0)]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(&table1)
        .column((&table1, "name"))
        .column((&table2, "is_cat"))
        .inner_join(
            table2
                .as_str()
                .on((table1.as_str(), "id").equals(Column::from((&table2, "t1_id")))),
        )
        .order_by("id".ascend());

    let res = api.conn().select(query).await?;

    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());
    assert_eq!(Some(true), row["is_cat"].as_bool());

    let row = res.get(1).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());
    assert_eq!(Some(false), row["is_cat"].as_bool());

    Ok(())
}

#[test_each_connector]
async fn table_inner_join(api: &mut dyn TestApi) -> crate::Result<()> {
    let table1 = api.create_temp_table("id int, name varchar(255)").await?;
    let table2 = api.create_temp_table("t1_id int, is_cat int").await?;
    let table3 = api.create_temp_table("id int, foo int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"])
        .values(vec![Value::int32(1), Value::int32(1)])
        .values(vec![Value::int32(2), Value::int32(0)]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table3, vec!["id", "foo"]).values(vec![Value::int32(1), Value::int32(1)]);

    api.conn().insert(insert.into()).await?;

    let joined_table = Table::from(&table1).inner_join(
        table2
            .as_str()
            .on((table1.as_str(), "id").equals(Column::from((&table2, "t1_id")))),
    );

    let query = Select::from_table(joined_table)
        // Select from a third table to ensure that the JOIN is specifically applied on the table1
        .and_from(&table3)
        .column((&table1, "name"))
        .column((&table2, "is_cat"))
        .column((&table3, "foo"))
        .order_by(Column::from((&table1, "id")).ascend());

    let res = api.conn().select(query).await?;

    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());
    assert_eq!(Some(true), row["is_cat"].as_bool());
    assert_eq!(Some(true), row["foo"].as_bool());

    let row = res.get(1).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());
    assert_eq!(Some(false), row["is_cat"].as_bool());
    assert_eq!(Some(true), row["foo"].as_bool());

    Ok(())
}

#[test_each_connector]
async fn left_join(api: &mut dyn TestApi) -> crate::Result<()> {
    let table1 = api.create_temp_table("id int, name varchar(255)").await?;
    let table2 = api.create_temp_table("t1_id int, is_cat int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"]).values(vec![Value::int32(1), Value::int32(1)]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(&table1)
        .column((&table1, "name"))
        .column((&table2, "is_cat"))
        .left_join(
            table2
                .as_str()
                .on((&table1, "id").equals(Column::from((&table2, "t1_id")))),
        )
        .order_by("id".ascend());

    let res = api.conn().select(query).await?;

    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());
    assert_eq!(Some(true), row["is_cat"].as_bool());

    let row = res.get(1).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());
    assert_eq!(None, row["is_cat"].as_bool());

    Ok(())
}

#[test_each_connector]
async fn table_left_join(api: &mut dyn TestApi) -> crate::Result<()> {
    let table1 = api.create_temp_table("id int, name varchar(255)").await?;
    let table2 = api.create_temp_table("t1_id int, is_cat int").await?;
    let table3 = api.create_temp_table("id int, foo int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"]).values(vec![Value::int32(1), Value::int32(1)]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table3, vec!["id", "foo"]).values(vec![Value::int32(1), Value::int32(1)]);

    api.conn().insert(insert.into()).await?;

    let joined_table = Table::from(&table1).left_join(
        table2
            .as_str()
            .on((&table1, "id").equals(Column::from((&table2, "t1_id")))),
    );

    let query = Select::from_table(joined_table)
        // Select from a third table to ensure that the JOIN is specifically applied on the table1
        .and_from(&table3)
        .column((&table1, "name"))
        .column((&table2, "is_cat"))
        .column((&table3, "foo"))
        .order_by(Column::from((&table1, "id")).ascend());

    let res = api.conn().select(query).await?;

    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());
    assert_eq!(Some(true), row["is_cat"].as_bool());
    assert_eq!(Some(true), row["foo"].as_bool());

    let row = res.get(1).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());
    assert_eq!(None, row["is_cat"].as_bool());
    assert_eq!(Some(true), row["foo"].as_bool());

    Ok(())
}

#[test_each_connector]
async fn limit_no_offset(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(&table).order_by("id".descend()).limit(1);

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn offset_no_limit(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id".descend()).offset(1);

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn limit_with_offset(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")])
        .values(vec![Value::int32(3), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id".ascend()).limit(1).offset(2);

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn limit_with_offset_no_given_order(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::int32(1), Value::text("Musti")])
        .values(vec![Value::int32(2), Value::text("Naukio")])
        .values(vec![Value::int32(3), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).limit(1).offset(2);

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_default_value_insert(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int default 1, name varchar(255) default 'Musti'")
        .await?;

    let changes = api.conn().execute(Insert::single_into(&table).into()).await?;
    assert_eq!(1, changes);

    let select = Select::from_table(&table);

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[cfg(any(feature = "mssql", feature = "postgresql", feature = "sqlite"))]
#[test_each_connector(tags("mssql", "postgresql", "sqlite"))]
async fn returning_insert(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.get_name();

    api.conn()
        .raw_cmd(&format!("CREATE TABLE {table} (id int primary key, name varchar(255))"))
        .await?;

    let insert = Insert::single_into(&table).value("id", 1).value("name", "Naukio");

    let res = api
        .conn()
        .insert(
            Insert::from(insert)
                .returning(vec!["id", "name"])
                .comment("this should be ignored"),
        )
        .await;

    api.conn().raw_cmd(&format!("DROP TABLE {table}")).await?;

    let res = res?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "sqlite"))]
#[test_each_connector(tags("postgresql", "sqlite"))]
async fn returning_update(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.get_name();

    api.conn()
        .raw_cmd(&format!("CREATE TABLE {table} (id int primary key, name varchar(255))"))
        .await?;

    api.conn()
        .insert(
            Insert::single_into(&table)
                .value("id", 1)
                .value("name", "Naukio")
                .into(),
        )
        .await?;

    let res = api
        .conn()
        .query(
            Update::table(&table)
                .set("name", "Updated")
                .returning(vec!["id", "name"])
                .comment("this should be ignored")
                .into(),
        )
        .await;

    api.conn().raw_cmd(&format!("DROP TABLE {table}")).await?;

    let res = res?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Updated"), row["name"].as_str());

    Ok(())
}

#[cfg(all(feature = "mssql", feature = "bigdecimal"))]
#[test_each_connector(tags("mssql"))]
async fn returning_decimal_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let dec = BigDecimal::from_str("17661757261711787211853")?;
    let table = api.create_temp_table("id int, val numeric(26,0)").await?;
    let col = Column::from("val").type_family(TypeFamily::Decimal(Some((26, 0))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, dec.clone());

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some(&dec), row["val"].as_numeric());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_constant_nvarchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, val nvarchar(4000)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Constant(4000))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_max_nvarchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, val nvarchar(max)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Maximum)));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_constant_varchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, val varchar(4000)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Constant(4000))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_max_varchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, val varchar(max)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Maximum)));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn multiple_resultset_should_return_the_last_one(api: &mut dyn TestApi) -> crate::Result<()> {
    let res = api
        .conn()
        .query_raw("SELECT 1 AS foo; SELECT 1 AS foo, 2 AS bar;", &[])
        .await?;

    assert_eq!(&vec!["foo", "bar"], res.columns());

    let row = res.into_single()?;

    assert_eq!(Some(&Value::int32(1)), row.get("foo"));
    assert_eq!(Some(&Value::int32(2)), row.get("bar"));

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_temp_table(&format!("id int, name varchar(255), {constraint}"))
        .await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let table = Table::from(&table_name).add_unique_index("id");
    let cols = vec![(&table_name, "id"), (&table_name, "name")];

    let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
        .values(vec![val!(1), val!("Naukio")])
        .values(vec![val!(2), val!("Belka")])
        .into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let res = api.conn().select(Select::from_table(table)).await?;
    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_temp_table(&format!("id int default 10, name varchar(255), {constraint}"))
        .await?;

    let insert = Insert::single_into(&table_name).value("id", 10).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").default(10);
    let table = Table::from(&table_name).add_unique_index(id);

    let insert: Insert<'_> = Insert::single_into(table.clone()).value("name", "Naukio").into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(0, changes);

    let select = Select::from_table(table);

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(10), row["id"].as_i32());

    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique_with_autogen_default(
    api: &mut dyn TestApi,
) -> crate::Result<()> {
    let table_name = api
        .create_temp_table(&format!("{}, name varchar(255)", api.autogen_id("id")))
        .await?;

    let id = Column::from("id").default(DefaultValue::Generated);
    let table = Table::from(&table_name).add_unique_index(id);

    let insert: Insert<'_> = Insert::single_into(table.clone()).value("name", "Naukio").into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let select = Select::from_table(table);

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[cfg(any(feature = "mssql", feature = "postgresql"))]
#[test_each_connector(tags("postgresql", "mssql"))]
async fn single_insert_conflict_do_nothing_with_returning(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_temp_table(&format!("id int, name varchar(255), {constraint}"))
        .await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let table = Table::from(&table_name).add_unique_index("id");
    let cols = vec![(&table_name, "id"), (&table_name, "name")];

    let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
        .values(vec![val!(1), val!("Naukio")])
        .values(vec![val!(2), val!("Belka")])
        .into();

    let res = api
        .conn()
        .insert(insert.on_conflict(OnConflict::DoNothing).returning(vec!["name"]))
        .await?;

    assert_eq!(1, res.len());
    assert_eq!(1, res.columns().len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_two_uniques(api: &mut dyn TestApi) -> crate::Result<()> {
    let id_constraint = api.unique_constraint("id");
    let name_constraint = api.unique_constraint("name");

    let table_name = api
        .create_temp_table(&format!(
            "id int, name varchar(255), {id_constraint}, {name_constraint}"
        ))
        .await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let table = Table::from(&table_name).add_unique_index("id").add_unique_index("name");

    let cols = vec![(&table_name, "id"), (&table_name, "name")];

    let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
        .values(vec![val!(1), val!("Naukio")])
        .values(vec![val!(3), val!("Musti")])
        .values(vec![val!(2), val!("Belka")])
        .into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_two_uniques_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let id_constraint = api.unique_constraint("id");
    let name_constraint = api.unique_constraint("name");

    let table_name = api
        .create_temp_table(&format!(
            "id int, name varchar(255) default 'Musti', {id_constraint}, {name_constraint}"
        ))
        .await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").table(&table_name);
    let name = Column::from("name").default("Musti").table(&table_name);

    let table = Table::from(&table_name)
        .add_unique_index(id.clone())
        .add_unique_index(name.clone());

    let insert: Insert<'_> = Insert::single_into(table.clone()).value(id, 2).into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(0, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compound_unique(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_temp_table("id int, name varchar(255)").await?;
    api.create_index(&table_name, "id asc, name asc").await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").table(&table_name);
    let name = Column::from("name").table(&table_name);

    let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

    let insert: Insert<'_> = Insert::multi_into(table.clone(), vec![id, name])
        .values(vec![val!(1), val!("Musti")])
        .values(vec![val!(1), val!("Naukio")])
        .into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compound_unique_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api
        .create_temp_table("id int, name varchar(255) default 'Musti'")
        .await?;
    api.create_index(&table_name, "id asc, name asc").await?;

    let insert = Insert::single_into(&table_name).value("id", 1).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").table(&table_name);
    let name = Column::from("name").table(&table_name).default("Musti");

    let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

    let insert: Insert<'_> = Insert::single_into(table.clone()).value(id, 1).into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(0, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());

    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_unique_with_autogen(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api
        .create_temp_table(&format!("{}, name varchar(100)", api.autogen_id("id")))
        .await?;

    let insert = Insert::single_into(&table_name).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").table(&table_name).default(DefaultValue::Generated);
    let name = Column::from("name").table(&table_name);

    let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);
    let insert: Insert<'_> = Insert::single_into(table.clone()).value(name, "Naukio").into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compound_unique_with_autogen_default(
    api: &mut dyn TestApi,
) -> crate::Result<()> {
    let table_name = api
        .create_temp_table(&format!("{}, name varchar(100) default 'Musti'", api.autogen_id("id")))
        .await?;

    api.create_index(&table_name, "id asc, name asc").await?;

    let insert = Insert::single_into(&table_name).value("name", "Musti");
    api.conn().insert(insert.into()).await?;

    let id = Column::from("id").table(&table_name).default(DefaultValue::Generated);
    let name = Column::from("name").table(&table_name).default("Musti");

    let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

    let insert: Insert<'_> = Insert::single_into(table.clone()).value(name, "Musti").into();

    let changes = api
        .conn()
        .execute(insert.on_conflict(OnConflict::DoNothing).into())
        .await?;

    assert_eq!(1, changes);

    let select = Select::from_table(table).order_by("id".ascend());

    let res = api.conn().select(select).await?;
    assert_eq!(2, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i32());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn updates(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::single_into(&table_name).value("name", "Musti").value("id", 1);
    api.conn().insert(insert.into()).await?;

    let update = Update::table(&table_name).set("name", "Naukio").so_that("id".equals(1));
    let changes = api.conn().execute(update.into()).await?;

    assert_eq!(1, changes);

    let select = Select::from_table(&table_name).order_by("id".ascend());
    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn deletes(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_temp_table("id int, name varchar(255)").await?;

    let insert = Insert::single_into(&table_name).value("name", "Musti").value("id", 1);
    api.conn().insert(insert.into()).await?;

    let delete = Delete::from_table(&table_name).so_that("id".equals(1));
    let changes = api.conn().execute(delete.into()).await?;

    assert_eq!(1, changes);

    let select = Select::from_table(&table_name).order_by("id".ascend());
    let res = api.conn().select(select).await?;
    assert_eq!(0, res.len());

    Ok(())
}

// TODO: Figure out why it doesn't work on MariaDB
// Error { kind: QueryError(Server(ServerError { code: 1115, message: "Unknown character set: 'gb18030'", state: "42000" })), original_code: Some("1115"), original_message: Some("Unknown character set: 'gb18030'") }
#[test_each_connector(tags("mysql"), ignore("mysql_mariadb"))]
async fn text_columns_with_non_utf8_encodings_can_be_queried(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id integer auto_increment primary key, value varchar(100) character set gb18030")
        .await?;

    let insert = Insert::multi_into(&table, vec!["value"])
        .values(vec!["法式咸派"])
        .values(vec!["土豆"]);

    api.conn().insert(insert.into()).await?;

    let select = Select::from_table(&table).column("value");
    let rows = api.conn().select(select).await?;

    let row = rows.get(0).unwrap();
    let res = row.get("value").unwrap().as_str();
    assert_eq!(Some("法式咸派"), res,);

    let row = rows.get(1).unwrap();
    let res = row.get("value").unwrap().as_str();
    assert_eq!(Some("土豆"), res);

    Ok(())
}

// TODO: Figure out why it doesn't work on mariadb
#[test_each_connector(tags("mysql"), ignore("mysql_mariadb"))]
async fn filtering_by_json_values_does_not_work_but_does_not_crash(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int4 auto_increment primary key, nested json not null")
        .await?;

    let insert = Insert::multi_into(&table, ["nested"])
        .values(vec!["{\"isTrue\": true}"])
        .values(vec!["{\"isTrue\": false}"]);

    api.conn().query(insert.into()).await?;

    let select = Select::from_table(&table).so_that("nested".equals("{\"isTrue\": false}"));
    let result = api.conn().query(select.into()).await?;

    assert!(result.is_empty());

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn float_columns_cast_to_f32(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int4 auto_increment primary key, f float not null")
        .await?;

    let insert = Insert::single_into(&table).value("f", 6.4123456);
    api.conn().query(insert.into()).await?;

    let select = Select::from_table(&table).column("f");
    let row = api.conn().query(select.into()).await?.into_single()?;
    let value = row.at(0).unwrap();

    assert_eq!(Some(6.4123454), value.as_f32());

    Ok(())
}

// TODO: Figure out why it doesn't work on MySQL8
//panicked at 'assertion failed: `(left == right)`
// left: `Numeric(Some(BigDecimal("1.0")))`,
// right: `Double(Some(1.0))`'
#[test_each_connector(tags("mysql"), ignore("mysql8"))]

async fn newdecimal_conversion_is_handled_correctly(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(sum(Value::int32(1)).alias("theone"));
    let result = api.conn().select(select).await?;

    assert_eq!(Value::numeric("1.0".parse().unwrap()), result.into_single().unwrap()[0]);

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn unsigned_integers_are_handled(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int4 auto_increment primary key, big bigint unsigned")
        .await?;

    let insert = Insert::multi_into(&table, ["big"])
        .values((2,))
        .values((std::i64::MAX,));
    api.conn().insert(insert.into()).await?;

    let select = Select::from_table(&table).column("big").order_by("id");
    let roundtripped = api.conn().select(select).await?;

    let expected = &[2, std::i64::MAX];
    let actual: Vec<i64> = roundtripped
        .into_iter()
        .map(|row| row.at(0).unwrap().as_i64().unwrap())
        .collect();

    assert_eq!(actual, expected);

    Ok(())
}

#[test_each_connector(tags("mysql", "postgresql"))]
async fn json_filtering_works(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };

    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": "a" }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": "b" }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;

    // Equals
    {
        let select = Select::from_table(&table).so_that(Column::from("obj").equals(serde_json::json!({ "a": "b" })));
        let result = api.conn().select(select).await?;

        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(2), row["id"].as_i32());
    }

    // Not equals
    {
        let select =
            Select::from_table(&table).so_that(Column::from("obj").not_equals(serde_json::json!({ "a": "a" })));

        let result = api.conn().query(select.into()).await?;

        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(2), row["id"].as_i32());
    }

    Ok(())
}

#[test_each_connector(tags("mssql", "postgresql"))]
async fn xml_filtering_works(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, xmlfield {}", api.autogen_id("id"), "xml"))
        .await?;

    let one = Insert::single_into(&table).value("xmlfield", Value::xml("<pig>oink</pig>"));
    let two = Insert::single_into(&table).value("xmlfield", Value::xml("<horse>neigh</horse>"));

    api.conn().insert(one.into()).await?;
    api.conn().insert(two.into()).await?;

    // Equals
    {
        let select =
            Select::from_table(&table).so_that(Column::from("xmlfield").equals(Value::xml("<horse>neigh</horse>")));

        let result = api.conn().select(select).await?;
        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(2), row["id"].as_i32());
    }

    // Not equals
    {
        let select =
            Select::from_table(&table).so_that(Column::from("xmlfield").not_equals(Value::xml("<horse>neigh</horse>")));
        let result = api.conn().query(select.into()).await?;
        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(1), row["id"].as_i32());
    }

    Ok(())
}

#[test_each_connector]
async fn upper_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(upper("foo").alias("val"));
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some("FOO"), row["val"].as_str());

    Ok(())
}

#[test_each_connector]
async fn lower_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(lower("BAR").alias("val"));
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some("bar"), row["val"].as_str());

    Ok(())
}

#[test_each_connector]
async fn op_test_add_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 1).value("b", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") + col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(3), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_add_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 3).value("c", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") + val!(col!("b") + col!("c")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(7), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(1), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_three_items(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 1).value("c", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b") - col!("c"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(0), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 3).value("c", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - val!(col!("b") + col!("c")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(-2), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_mul_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int").await?;

    let insert = Insert::single_into(&table).value("a", 6);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") * col!("a"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(36), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_mul_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 6).value("b", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") * (col!("a") - col!("b")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(30), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_multiple_operations(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 4).value("b", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b") * col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(0), row[0].as_integer());

    Ok(())
}

#[test_each_connector]
async fn op_test_div_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("a real, b real").await?;

    let insert = Insert::single_into(&table).value("a", 6.0).value("b", 3.0);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") / col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    match api.system() {
        "mssql" | "postgres" => assert_eq!(Some(2.0), row[0].as_f32()),
        _ => assert_eq!(Some(2.0), row[0].as_f64()),
    }

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn enum_values(api: &mut dyn TestApi) -> crate::Result<()> {
    let type_name = api.get_name();
    let create_type = format!("CREATE TYPE {} AS ENUM ('A', 'B')", &type_name);
    api.conn().raw_cmd(&create_type).await?;

    let table = api
        .create_temp_table(&format!("id SERIAL PRIMARY KEY, value {}", &type_name))
        .await?;

    api.conn()
        .insert(
            Insert::single_into(&table)
                .value(
                    "value",
                    Value::enum_variant_with_name("A", EnumName::new(&type_name, Option::<String>::None)),
                )
                .into(),
        )
        .await?;

    api.conn()
        .insert(
            Insert::single_into(&table)
                .value(
                    "value",
                    Value::enum_variant_with_name("B", EnumName::new(&type_name, Option::<String>::None)),
                )
                .into(),
        )
        .await?;

    api.conn()
        .insert(Insert::single_into(&table).value("value", Value::null_enum()).into())
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".ascend());
    let res = api.conn().select(select).await?;

    let row = res.get(0).unwrap();
    assert_eq!(Some(&Value::enum_variant("A")), row.at(0));

    let row = res.get(1).unwrap();
    assert_eq!(Some(&Value::enum_variant("B")), row.at(0));

    let row = res.get(2).unwrap();
    assert_eq!(Some(&Value::null_enum()), row.at(0));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
#[cfg(feature = "postgresql")]
async fn row_to_json_normal(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte = Select::default()
        .value(val!("hello_world").alias("toto"))
        .into_cte("one");
    let select = Select::from_table("one").value(row_to_json("one", false)).with(cte);
    let result = api.conn().select(select).await?;

    assert_eq!(
        Value::json(serde_json::json!({
            "toto": "hello_world"
        })),
        result.into_single().unwrap()[0]
    );

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
#[cfg(feature = "postgresql")]
async fn row_to_json_pretty(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte = Select::default()
        .value(val!("hello_world").alias("toto"))
        .into_cte("one");
    let select = Select::from_table("one").value(row_to_json("one", true)).with(cte);
    let result = api.conn().select(select).await?;

    assert_eq!(
        Value::json(serde_json::json!({
            "toto": "hello_world"
        })),
        result.into_single().unwrap()[0]
    );

    Ok(())
}

#[test_each_connector(ignore("mysql"))]
async fn single_common_table_expression(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte = Select::default()
        .value(val!(1).alias("val"))
        .into_cte("one")
        .column("val");

    let select = Select::from_table("one").column("val").with(cte);

    let res = api.conn().select(select).await?;
    let row = res.get(0).unwrap();

    if api.connector_tag().intersects(Tags::POSTGRES) {
        assert_eq!(Some(&Value::text("1")), row.at(0));
    } else if api.connector_tag().intersects(Tags::SQLITE) {
        // NOTE: with explicit values, SQLite does not pass the specific declaration type, so is assumed int64
        assert_eq!(Some(&Value::int64(1)), row.at(0));
    } else {
        assert_eq!(Some(&Value::int32(1)), row.at(0));
    }

    Ok(())
}

#[test_each_connector(ignore("mysql"))]
async fn multiple_common_table_expressions(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte_1 = Select::default()
        .value(val!(1).alias("val"))
        .into_cte("one")
        .column("val");

    let cte_2 = Select::default()
        .value(val!(2).alias("val"))
        .into_cte("two")
        .column("val");

    let select = Select::from_table("one")
        .with(cte_1)
        .with(cte_2)
        .inner_join("two")
        .column(("one", "val"))
        .column(("two", "val"));

    let res = api.conn().select(select).await?;
    let row = res.get(0).unwrap();

    if api.connector_tag().intersects(Tags::POSTGRES) {
        assert_eq!(Some(&Value::text("1")), row.at(0));
        assert_eq!(Some(&Value::text("2")), row.at(1));
    } else if api.connector_tag().intersects(Tags::SQLITE) {
        // NOTE: with explicit values, SQLite does not pass the specific declaration type, so is assumed int64
        assert_eq!(Some(&Value::int64(1)), row.at(0));
        assert_eq!(Some(&Value::int64(2)), row.at(1));
    } else {
        assert_eq!(Some(&Value::int32(1)), row.at(0));
        assert_eq!(Some(&Value::int32(2)), row.at(1));
    }

    Ok(())
}

// A query where we compare a tuple against a query that returns tuples
// of same size, the inclusive version.
//
// e.g.
//
// WHERE (a, b) IN (SELECT x, y FROM ..);
//
// SQL Server doesn't support tuple comparison, so the query is modified into a
// common table expression, but should still return the same result.
#[test_each_connector]
async fn compare_tuple_in_select(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 varchar(3), id2 varchar(3)").await?;

    let insert = Insert::single_into(&table).value("id1", "foo").value("id2", "bar");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table).value("id1", "omg").value("id2", "lol");
    api.conn().insert(insert.into()).await?;

    // Table has values of:
    //
    // | id1 | id2 |
    // +-----+-----+
    // | foo | bar |
    // | omg | lol |

    let sel_1 = Select::default()
        .value(val!("foo").alias("a"))
        .value(val!("bar").alias("b"));

    let sel_2 = Select::default()
        .value(val!("mus").alias("a"))
        .value(val!("pus").alias("b"));

    let union = Union::new(sel_1).all(sel_2);

    let id1 = Column::new("id1").table(table.as_str());
    let id2 = Column::new("id2").table(table.as_str());

    let row = Row::from(vec![id1, id2]);
    let select = Select::from_table(&table).so_that(row.in_selection(union));

    // WHERE (id1, id2) IN (SELECT 'foo' AS a, 'bar' AS b UNION ALL SELECT 'mus' AS a, 'pus' AS b)

    let res = api.conn().select(select).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some(&Value::text("foo")), row.at(0));
    assert_eq!(Some(&Value::text("bar")), row.at(1));

    Ok(())
}

// A query where we compare a tuple against a query that returns tuples
// of same size, the non-inclusive version.
//
// e.g.
//
// WHERE (a, b) NOT IN (SELECT x, y FROM ..);
#[test_each_connector]
async fn compare_tuple_not_in_select(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id1 varchar(3), id2 varchar(3)").await?;

    let insert = Insert::single_into(&table).value("id1", "foo").value("id2", "bar");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table).value("id1", "omg").value("id2", "lol");
    api.conn().insert(insert.into()).await?;

    // Table has values of:
    //
    // | id1 | id2 |
    // +-----+-----+
    // | foo | bar |
    // | omg | lol |

    let sel_1 = Select::default()
        .value(val!("foo").alias("a"))
        .value(val!("bar").alias("b"));

    let sel_2 = Select::default()
        .value(val!("mus").alias("a"))
        .value(val!("pus").alias("b"));

    let union = Union::new(sel_1).all(sel_2);

    let id1 = Column::new("id1").table(table.as_str());
    let id2 = Column::new("id2").table(table.as_str());

    let row = Row::from(vec![id1, id2]);
    let select = Select::from_table(&table).so_that(row.not_in_selection(union));

    // WHERE (id1, id2) NOT IN (SELECT 'foo' AS a, 'bar' AS b UNION ALL SELECT 'mus' AS a, 'pus' AS b)

    let res = api.conn().select(select).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some(&Value::text("omg")), row.at(0));
    assert_eq!(Some(&Value::text("lol")), row.at(1));

    Ok(())
}

#[test_each_connector]
async fn join_with_compound_columns(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_1 = api.create_temp_table("id1 int, id2 int, data varchar(3)").await?;
    let table_2 = api.create_temp_table("id3 int, id4 int").await?;

    let insert = Insert::single_into(&table_1)
        .value("id1", 1)
        .value("id2", 2)
        .value("data", "foo");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_1)
        .value("id1", 2)
        .value("id2", 3)
        .value("data", "bar");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_2).value("id3", 1).value("id4", 2);
    api.conn().insert(insert.into()).await?;

    let left_row = Row::from(vec![col!("id1"), col!("id2")]);
    let right_row = Row::from(vec![col!("id3"), col!("id4")]);

    let join = table_2.as_str().on(left_row.equals(right_row));

    let select = Select::from_table(&table_1)
        .column("id1")
        .column("id2")
        .inner_join(join);

    let res = api.conn().select(select).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some(&Value::int32(1)), row.at(0));
    assert_eq!(Some(&Value::int32(2)), row.at(1));

    Ok(())
}

#[test_each_connector]
async fn join_with_non_matching_compound_columns(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_1 = api.create_temp_table("id1 int, id2 int, data varchar(3)").await?;
    let table_2 = api.create_temp_table("id3 int, id4 int").await?;

    let insert = Insert::single_into(&table_1)
        .value("id1", 1)
        .value("id2", 2)
        .value("data", "foo");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_1)
        .value("id1", 2)
        .value("id2", 3)
        .value("data", "bar");
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_2).value("id3", 1).value("id4", 2);
    api.conn().insert(insert.into()).await?;

    let left_row = Row::from(vec![col!("id1"), col!("id2")]);
    let right_row = Row::from(vec![col!("id3"), col!("id4")]);

    let join = table_2.as_str().on(left_row.not_equals(right_row));

    let select = Select::from_table(&table_1)
        .column("id1")
        .column("id2")
        .inner_join(join);

    let res = api.conn().select(select).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();

    assert_eq!(Some(&Value::int32(2)), row.at(0));
    assert_eq!(Some(&Value::int32(3)), row.at(1));

    Ok(())
}

#[test_each_connector(ignore("sqlite"))]
async fn insert_default_keyword(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int, value int DEFAULT 1").await?;

    let insert = Insert::single_into(&table)
        .value("value", default_value())
        .value("id", 4);

    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::int32(4), row["id"]);
    assert_eq!(Value::int32(1), row["value"]);

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn ints_read_write_to_numeric(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let table = api.create_temp_table("id int, value numeric(12,2)").await?;

    let insert = Insert::multi_into(&table, ["id", "value"])
        .values(vec![Value::int32(1), Value::double(1234.5)])
        .values(vec![Value::int32(2), Value::int32(1234)])
        .values(vec![Value::int32(3), Value::int32(12345)]);

    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let rows = api.conn().select(select).await?;

    for (i, row) in rows.into_iter().enumerate() {
        match i {
            0 => assert_eq!(Value::numeric(BigDecimal::from_str("1234.5").unwrap()), row["value"]),
            1 => assert_eq!(Value::numeric(BigDecimal::from_str("1234.0").unwrap()), row["value"]),
            _ => assert_eq!(Value::numeric(BigDecimal::from_str("12345.0").unwrap()), row["value"]),
        }
    }

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn bigdecimal_read_write_to_floating(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let table = api.create_temp_table("id int, a float4, b float8").await?;
    let val = BigDecimal::from_str("0.1").unwrap();

    let insert = Insert::multi_into(&table, ["id", "a", "b"]).values(vec![
        Value::int32(1),
        Value::numeric(val.clone()),
        Value::numeric(val.clone()),
    ]);

    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::float(0.1), row["a"]);
    assert_eq!(Value::double(0.1), row["b"]);

    Ok(())
}

#[test_each_connector]
async fn coalesce_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let exprs: Vec<Expression> = vec![Value::null_text().into(), Value::text("Individual").into()];
    let select = Select::default().value(coalesce(exprs).alias("val"));
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some("Individual"), row["val"].as_str());

    Ok(())
}

fn value_into_json(value: &Value) -> Option<serde_json::Value> {
    match value.typed.clone() {
        // MariaDB returns JSON as text
        ValueType::Text(Some(text)) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| panic!("expected parsable text to json, found {}", text));

            Some(json)
        }
        ValueType::Json(Some(json)) => Some(json),
        _ => None,
    }
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_extract_path_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj json", api.autogen_id("id")))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": "c" } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a\":{": "b" }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.a.b"), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!("c")));
    let mut res = api.conn().select(select).await?.into_iter();

    // Test object extraction
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": "c" } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.a.b[1]"), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!(2)));
    let mut res = api.conn().select(select).await?.into_iter();

    // Test array index extraction
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.\"a\\\":{\""), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!("b")));
    let mut res = api.conn().select(select).await?.into_iter();

    // Test escaped chars in keys
    assert_eq!(
        Some(serde_json::json!({ "a\":{": "b" })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    Ok(())
}

#[cfg(feature = "postgresql")]
async fn json_extract_array_path_postgres(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": "c" } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a\":{": "b" }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    // Test object extraction
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals("\"c\""));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": "c" } })),
        value_into_json(&row["obj"])
    );

    // Test equality with Json value
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!("c")));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": "c" } })),
        value_into_json(&row["obj"])
    );

    // Test array index extraction
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b", "1"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!(2)));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        value_into_json(&row["obj"])
    );

    // Test escaped chars in keys
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a\":{"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::json!("b")));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(Some(serde_json::json!({ "a\":{": "b" })), value_into_json(&row["obj"]));

    // Test equality with Json value with `extract_as_string: true`
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b"]), true).into();
    let select = Select::from_table(&table).so_that(extract.equals("c"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": "c" } })),
        value_into_json(&row["obj"])
    );

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_extract_array_path_fun_on_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_extract_array_path_postgres(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_extract_array_path_fun_on_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_extract_array_path_postgres(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_contains(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": ["foo", "bar"] } }));
    let fourth_insert = Insert::single_into(&table).value(
        "obj",
        serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } }),
    );

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;
    api.conn().insert(fourth_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert contains number
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains(serde_json::json!([2])));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        value_into_json(&row["obj"])
    );

    // Assert contains string
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains(serde_json::json!(["bar"])));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        value_into_json(&row["obj"])
    );

    // Assert contains object
    let select =
        Select::from_table(&table).so_that(path.clone().json_array_contains(serde_json::json!([{ "bar": "foo" }])));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        value_into_json(&row["obj"])
    );

    // Assert contains array
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains(serde_json::json!([[1, 2]])));
    let mut res = api.conn().select(select).await?.into_iter();

    // MariaDB doesn't support finding arrays of arrays
    if api.connector_tag().intersects(Tags::MYSQL_MARIADB) {
        assert_eq!(
            Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
            value_into_json(&res.next().unwrap()["obj"])
        );
        assert_eq!(
            Some(serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
            value_into_json(&res.next().unwrap()["obj"])
        );
        assert_eq!(None, res.next());
    } else {
        assert_eq!(
            Some(serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
            value_into_json(&res.next().unwrap()["obj"])
        );
        assert_eq!(None, res.next());
    }

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_contains_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_contains(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_contains_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_contains(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_contains_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_contains(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_not_contains(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [2, 3] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [4, 5] } }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert NOT contains number
    let select = Select::from_table(&table).so_that(path.clone().json_array_not_contains("[2]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [4, 5] } })),
        value_into_json(&row["obj"])
    );

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_contains_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_contains(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_contains_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_contains(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_not_contains_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_contains(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_begins_with(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": ["foo", "bar"] } }));
    let fourth_insert = Insert::single_into(&table).value(
        "obj",
        serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } }),
    );

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;
    api.conn().insert(fourth_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert starts with number
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with(serde_json::json!(1)));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert starts with string
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with(serde_json::json!("foo")));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert starts with object
    let select =
        Select::from_table(&table).so_that(path.clone().json_array_begins_with(serde_json::json!({ "foo": "bar" })));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert starts with array
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with(serde_json::json!([1, 2])));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_begins_with_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_begins_with(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_begins_with_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_begins_with(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_begins_with_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_begins_with(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_not_begins_with(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 3] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [4, 5] } }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert NOT starts with number
    let select = Select::from_table(&table).so_that(path.clone().json_array_not_begins_with(serde_json::json!(1)));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [4, 5] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_begins_with_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_begins_with(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_begins_with_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_begins_with(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_not_begins_with_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_begins_with(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_ends_into(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": ["foo", "bar"] } }));
    let fourth_insert = Insert::single_into(&table).value(
        "obj",
        serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } }),
    );

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;
    api.conn().insert(fourth_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert ends with number
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into(serde_json::json!(3)));
    let mut res = api.conn().select(select).await?.into_iter();

    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert ends with string
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into(serde_json::json!("bar")));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert ends with object
    let select =
        Select::from_table(&table).so_that(path.clone().json_array_ends_into(serde_json::json!({ "bar": "foo" })));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    // Assert ends with array
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into(serde_json::json!([3, 4])));
    let mut res = api.conn().select(select).await?.into_iter();
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_ends_into_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_ends_into(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_ends_into_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_ends_into(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_ends_into_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_ends_into(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_array_not_ends_into(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2] } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [3, 2] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [4, 5] } }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("obj"), path.clone(), false).into();

    // Assert NOT starts with number
    let select = Select::from_table(&table).so_that(path.clone().json_array_not_ends_into(serde_json::json!(2)));
    let mut res = api.conn().select(select).await?.into_iter();

    assert_eq!(
        Some(serde_json::json!({ "a": { "b": [4, 5] } })),
        value_into_json(&res.next().unwrap()["obj"])
    );
    assert_eq!(None, res.next());

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_ends_into_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_ends_into(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_array_not_ends_into_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_ends_into(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_array_not_ends_into_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_array_not_ends_into(api, "json").await?;

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
async fn json_gt_gte_lt_lte(api: &mut dyn TestApi, json_type: &str) -> crate::Result<()> {
    let table = api
        .create_temp_table(&format!("{}, json {}", api.autogen_id("id"), json_type))
        .await?;

    let insert = Insert::single_into(&table).value("json", serde_json::json!({ "a": { "b": 1 } }));
    let second_insert = Insert::single_into(&table).value("json", serde_json::json!({ "a": { "b": 50 } }));
    let third_insert = Insert::single_into(&table).value("json", serde_json::json!({ "a": { "b": 100 } }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };
    let path: Expression = json_extract(col!("json"), path.clone(), false).into();

    // Assert JSON greater_than (CAST on right side)
    let select = Select::from_table(&table).so_that(path.clone().greater_than(Value::json(serde_json::json!(1))));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 100 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(None, res.get(2));

    // Assert JSON greater_than (CAST on left side)
    let json_value: Expression = Value::json(serde_json::json!(50)).into();
    let select = Select::from_table(&table).so_that(json_value.greater_than(path.clone()));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(None, res.get(1));

    // Assert JSON greater_than_or_equals (CAST on right side)
    let select =
        Select::from_table(&table).so_that(path.clone().greater_than_or_equals(Value::json(serde_json::json!(1))));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 100 } })),
        value_into_json(&res.get(2).unwrap()["json"])
    );
    assert_eq!(None, res.get(3));

    // Assert JSON greater_than_or_equals (CAST on left side)
    let json_value: Expression = Value::json(serde_json::json!(50)).into();
    let select = Select::from_table(&table).so_that(json_value.greater_than_or_equals(path.clone()));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(None, res.get(2));

    // Assert JSON less_than (CAST on right side)
    let select = Select::from_table(&table).so_that(path.clone().less_than(Value::json(serde_json::json!(100))));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(None, res.get(2));

    // Assert JSON less_than (CAST on left side)
    let json_value: Expression = Value::json(serde_json::json!(1)).into();
    let select = Select::from_table(&table).so_that(json_value.less_than(path.clone()));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 100 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(None, res.get(2));

    // Assert JSON less_than_or_equals (CAST on right side)
    let select =
        Select::from_table(&table).so_that(path.clone().less_than_or_equals(Value::json(serde_json::json!(100))));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 100 } })),
        value_into_json(&res.get(2).unwrap()["json"])
    );
    assert_eq!(None, res.get(3));

    // Assert JSON less_than_or_equals (CAST on left side)
    let json_value: Expression = Value::json(serde_json::json!(1)).into();
    let select = Select::from_table(&table).so_that(json_value.less_than_or_equals(path));
    let res = api.conn().select(select).await?;
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 1 } })),
        value_into_json(&res.get(0).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 50 } })),
        value_into_json(&res.get(1).unwrap()["json"])
    );
    assert_eq!(
        Some(serde_json::json!({ "a": { "b": 100 } })),
        value_into_json(&res.get(2).unwrap()["json"])
    );
    assert_eq!(None, res.get(3));

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_gt_gte_lt_lte_fun_pg_jsonb(api: &mut dyn TestApi) -> crate::Result<()> {
    json_gt_gte_lt_lte(api, "jsonb").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn json_gt_gte_lt_lte_fun_pg_json(api: &mut dyn TestApi) -> crate::Result<()> {
    json_gt_gte_lt_lte(api, "json").await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn json_gt_gte_lt_lte_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    json_gt_gte_lt_lte(api, "json").await?;

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn text_search_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("name varchar(255), ingredients varchar(255)")
        .await?;

    let insert_1 = Insert::single_into(&table)
        .value("name", "Chicken Curry")
        .value("ingredients", "Chicken, Curry, Rice");
    let insert_2 = Insert::single_into(&table)
        .value("name", "Caesar Salad")
        .value("ingredients", "Salad, Chicken, Parmesan, Caesar Sauce");
    api.conn().insert(insert_1.into()).await?;
    api.conn().insert(insert_2.into()).await?;

    // Search on multiple columns at the same time
    let search: Expression = text_search(&[col!("name"), col!("ingredients")]).into();
    let q = Select::from_table(&table).so_that(search.matches("chicken"));
    let res = api.conn().select(q).await?;
    let row_one = res.get(0).unwrap();
    let row_two = res.get(1).unwrap();

    assert_eq!(row_one["name"], Value::from("Chicken Curry"));
    assert_eq!(row_one["ingredients"], Value::from("Chicken, Curry, Rice"));
    assert_eq!(row_two["name"], Value::from("Caesar Salad"));
    assert_eq!(
        row_two["ingredients"],
        Value::from("Salad, Chicken, Parmesan, Caesar Sauce")
    );

    // Search on a single column
    let search: Expression = text_search(&[col!("name")]).into();
    let q = Select::from_table(&table).so_that(search.matches("chicken"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(row["name"], Value::from("Chicken Curry"));
    assert_eq!(row["ingredients"], Value::from("Chicken, Curry, Rice"));

    // Search on a single column with NOT
    let search: Expression = text_search(&[col!("name")]).into();
    let q = Select::from_table(&table).so_that(search.not_matches("salad"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(row["name"], Value::from("Chicken Curry"));
    assert_eq!(row["ingredients"], Value::from("Chicken, Curry, Rice"));

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn text_search_relevance_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("name varchar(255), ingredients varchar(255)")
        .await?;

    let insert_1 = Insert::single_into(&table)
        .value("name", "Chicken Curry")
        .value("ingredients", "Chicken, Curry, Rice");
    let insert_2 = Insert::single_into(&table)
        .value("name", "Caesar Salad")
        .value("ingredients", "Salad, Chicken, Parmesan, Caesar Sauce");
    api.conn().insert(insert_1.into()).await?;
    api.conn().insert(insert_2.into()).await?;

    // Compute search relevance on multiple columns at the same time
    let search: Expression = text_search_relevance(&[col!("name"), col!("ingredients")], "chicken").into();
    let q = Select::from_table(&table).value(search.alias("relevance"));
    let mut res = api.conn().select(q).await?.into_iter();

    assert_eq!(res.next().unwrap()["relevance"], Value::float(0.075990885));
    assert_eq!(res.next().unwrap()["relevance"], Value::float(0.06079271));
    assert_eq!(res.next(), None);

    // Search on a single column
    let search: Expression = text_search_relevance(&[col!("name")], "chicken").into();
    let q = Select::from_table(&table).value(search.alias("relevance"));
    let mut res = api.conn().select(q).await?.into_iter();

    assert_eq!(res.next().unwrap()["relevance"], Value::float(0.06079271));
    assert_eq!(res.next().unwrap()["relevance"], Value::float(0.0));
    assert_eq!(res.next(), None);

    Ok(())
}

#[test_each_connector]
async fn select_comment(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default()
        .value("foo")
        .comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::text("foo"), res[0]);

    Ok(())
}

#[test_each_connector]
async fn insert_comment(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("name varchar(255)").await?;

    let query = Insert::single_into(&table).value("name", "Chicken Curry");
    let insert =
        Insert::from(query).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    api.conn().insert(insert).await?;

    Ok(())
}

#[test_each_connector]
async fn update_comment(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("name varchar(255)").await?;

    let insert = Insert::single_into(&table).value("name", "Chicken Curry");
    api.conn().insert(insert.into()).await?;

    let update = Update::table(&table)
        .set("name", "Caesar Salad")
        .comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    let res = api.conn().update(update).await?;

    assert_eq!(res, 1);

    Ok(())
}

#[test_each_connector]
async fn delete_comment(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("name varchar(255)").await?;

    let insert = Insert::single_into(&table).value("name", "Chicken Curry");
    api.conn().insert(insert.into()).await?;

    let delete =
        Delete::from_table(&table).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    api.conn().delete(delete).await?;

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql8"))]
async fn generate_binary_uuid(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(uuid_to_bin());
    let res = api.conn().select(select).await?.into_single()?;
    let val = res.into_single()?;

    // If it is a byte type and has a value, it's a generated UUID.
    assert!(matches!(val.typed, ValueType::Bytes(x) if x.is_some()));

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql8"))]
async fn generate_swapped_binary_uuid(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(uuid_to_bin_swapped());
    let res = api.conn().select(select).await?.into_single()?;
    let val = res.into_single()?;

    // If it is a byte type and has a value, it's a generated UUID.
    assert!(matches!(val.typed, ValueType::Bytes(x) if x.is_some()));

    Ok(())
}

#[cfg(feature = "mysql")]
#[test_each_connector(tags("mysql"))]
async fn generate_native_uuid(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(native_uuid());
    let res = api.conn().select(select).await?.into_single()?;
    let val = res.into_single()?;

    // If it is a text type and has a value, it's a generated string UUID.
    assert!(matches!(val.typed, ValueType::Text(x) if x.is_some()));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn query_raw_typed_numeric(api: &mut dyn TestApi) -> crate::Result<()> {
    let res = api
        .conn()
        .query_raw_typed(
            r#"SELECT
                    $1::float4     AS i4tof4,
                    $2::float4     AS i8tof4,

                    $3::float8     AS i4tof8,
                    $4::float8     AS i8tof8,

                    $5::int4       AS f4toi4,
                    $6::int4       AS f8toi4,

                    $7::int8       AS f4toi8,
                    $8::int8       AS f8toi8,

                    $9::int4       AS texttoi4,
                    $10::int8      AS texttoi8,

                    $11::float4    AS texttof4,
                    $12::float8    AS texttof8
                "#,
            &[
                Value::int32(42),     // $1
                Value::int64(42),     // $2
                Value::int32(42),     // $3
                Value::int64(42),     // $4
                Value::float(42.51),  // $5
                Value::double(42.51), // $6
                Value::float(42.51),  // $7
                Value::double(42.51), // $8
                Value::text("42"),    // $9
                Value::text("42"),    // $10
                Value::text("42.51"), // $11
                Value::text("42.51"), // $12
            ],
        )
        .await?
        .into_single()?;

    assert_eq!(Value::float(42.0), res["i4tof4"]);
    assert_eq!(Value::float(42.0), res["i8tof4"]);

    assert_eq!(Value::double(42.0), res["i4tof8"]);
    assert_eq!(Value::double(42.0), res["i8tof8"]);

    assert_eq!(Value::int32(43), res["f4toi4"]);
    assert_eq!(Value::int32(43), res["f8toi4"]);

    assert_eq!(Value::int64(43), res["f4toi8"]);
    assert_eq!(Value::int64(43), res["f8toi8"]);

    assert_eq!(Value::int32(42), res["texttoi4"]);
    assert_eq!(Value::int64(42), res["texttoi8"]);

    assert_eq!(Value::float(42.51), res["texttof4"]);
    assert_eq!(Value::double(42.51), res["texttof8"]);

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn query_raw_typed_date(api: &mut dyn TestApi) -> crate::Result<()> {
    use chrono::DateTime;
    use std::str::FromStr;

    let res = api
        .conn()
        .query_raw_typed(
            r#"SELECT
                    ($1::timestamp - $2::interval)  AS texttointerval,
                    $3 = DATE_PART('year', $4::date) AS is_year_2023;
                "#,
            &[
                Value::text("2022-01-01 00:00:00"), // $1
                Value::text("1 year"),              // $2
                Value::int32(2022),                 // $3
                Value::text("2022-01-01"),          // $4
            ],
        )
        .await?
        .into_single()?;

    assert_eq!(
        Value::from(DateTime::from_str("2021-01-01T00:00:00Z").unwrap()),
        res["texttointerval"]
    );
    assert_eq!(Value::boolean(true), res["is_year_2023"]);

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn query_raw_typed_json(api: &mut dyn TestApi) -> crate::Result<()> {
    use serde_json::json;

    let res = api
        .conn()
        .query_raw_typed(
            r#"SELECT
                    $1                               as json,
                    $2::text                         as jsontotext,
                    $3->'b'                          as json_operator,
                    json_extract_path($4::json, 'b') as json_extract,
                    jsonb_extract_path($5, 'b')      as jsonb_extract
                   ;
                "#,
            &[
                Value::json(json!({ "a":1, "b":2})), // $1
                Value::json(json!({ "a":1, "b":2})), // $2
                Value::json(json!({ "a":1, "b":2})), // $3
                Value::json(json!({ "a":1, "b":2})), // $4
                Value::json(json!({ "a":1, "b":2})), // $5
            ],
        )
        .await?
        .into_single()?;

    assert_eq!(Value::json(json!({ "a":1, "b":2})), res["json"]);
    assert_eq!(Value::text("{\"a\": 1, \"b\": 2}"), res["jsontotext"]);
    assert_eq!(Value::json(json!(2)), res["json_operator"]);
    assert_eq!(Value::json(json!(2)), res["json_extract"]);
    assert_eq!(Value::json(json!(2)), res["jsonb_extract"]);

    Ok(())
}

#[test_each_connector]
async fn order_by_nulls_first_last(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("name varchar(255), age int").await?;

    let insert = Insert::single_into(&table).value("name", "a").value("age", 1);
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table)
        .value("name", "b")
        .value("age", Value::null_int32());
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table)
        .value("name", Value::null_text())
        .value("age", 2);
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table)
        .value("name", Value::null_text())
        .value("age", Value::null_text());
    api.conn().insert(insert.into()).await?;

    // name ASC NULLS FIRST
    let select = Select::from_table(table.clone()).order_by("name".ascend_nulls_first());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(1).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(2).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(3).unwrap()["name"], Value::text("b"));

    // name ASC NULLS LAST
    let select = Select::from_table(table.clone()).order_by("name".ascend_nulls_last());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(1).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(2).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(3).unwrap()["name"], Value::null_text());

    // name DESC NULLS FIRST
    let select = Select::from_table(table.clone()).order_by("name".descend_nulls_first());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(1).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(2).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(3).unwrap()["name"], Value::text("a"));

    // name ASC NULLS LAST
    let select = Select::from_table(table.clone()).order_by("name".descend_nulls_last());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(1).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(2).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(3).unwrap()["name"], Value::null_text());

    // name ASC NULLS FIRST, age ASC NULLS FIRST
    let select = Select::from_table(table.clone())
        .order_by("name".ascend_nulls_first())
        .order_by("age".ascend_nulls_first());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(0).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(1).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(1).unwrap()["age"], Value::int32(2));

    assert_eq!(res.get(2).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(2).unwrap()["age"], Value::int32(1));

    assert_eq!(res.get(3).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(3).unwrap()["age"], Value::null_int32());

    // name ASC NULLS LAST, age ASC NULLS LAST
    let select = Select::from_table(table.clone())
        .order_by("name".ascend_nulls_last())
        .order_by("age".ascend_nulls_last());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(0).unwrap()["age"], Value::int32(1));

    assert_eq!(res.get(1).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(1).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(2).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(2).unwrap()["age"], Value::int32(2));

    assert_eq!(res.get(3).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(3).unwrap()["age"], Value::null_int32());

    // name DESC NULLS FIRST, age DESC NULLS FIRST
    let select = Select::from_table(table.clone())
        .order_by("name".descend_nulls_first())
        .order_by("age".descend_nulls_first());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(0).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(1).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(1).unwrap()["age"], Value::int32(2));

    assert_eq!(res.get(2).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(2).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(3).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(3).unwrap()["age"], Value::int32(1));

    // name DESC NULLS LAST, age DESC NULLS LAST
    let select = Select::from_table(table.clone())
        .order_by("name".descend_nulls_last())
        .order_by("age".descend_nulls_last());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(0).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(1).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(1).unwrap()["age"], Value::int32(1));

    assert_eq!(res.get(2).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(2).unwrap()["age"], Value::int32(2));

    assert_eq!(res.get(3).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(3).unwrap()["age"], Value::null_int32());

    // name ASC NULLS LAST, age DESC NULLS FIRST
    let select = Select::from_table(table.clone())
        .order_by("name".ascend_nulls_last())
        .order_by("age".descend_nulls_first());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(0).unwrap()["age"], Value::int32(1));

    assert_eq!(res.get(1).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(1).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(2).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(2).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(3).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(3).unwrap()["age"], Value::int32(2));

    // name DESC NULLS FIRST, age ASC NULLS LAST
    let select = Select::from_table(table.clone())
        .order_by("name".descend_nulls_first())
        .order_by("age".ascend_nulls_last());
    let res = api.conn().select(select).await?;

    assert_eq!(res.get(0).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(0).unwrap()["age"], Value::int32(2));

    assert_eq!(res.get(1).unwrap()["name"], Value::null_text());
    assert_eq!(res.get(1).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(2).unwrap()["name"], Value::text("b"));
    assert_eq!(res.get(2).unwrap()["age"], Value::null_int32());

    assert_eq!(res.get(3).unwrap()["name"], Value::text("a"));
    assert_eq!(res.get(3).unwrap()["age"], Value::int32(1));

    Ok(())
}

#[test_each_connector]
async fn concat_expressions(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("firstname varchar(255), lastname varchar(255)")
        .await?;

    let insert = Insert::single_into(&table)
        .value("firstname", "John")
        .value("lastname", "Doe");

    api.conn().insert(insert.into()).await?;

    let concat: Expression<'_> =
        concat::<'_, Expression<'_>>(vec![col!("firstname"), " ".into(), col!("lastname")]).into();
    let query = Select::from_table(table).value(concat.alias("concat"));

    let res = api.conn().select(query).await?.into_single()?;
    assert_eq!(res["concat"], Value::from("John Doe"));

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn all_in_expression(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int").await?;

    let insert = Insert::single_into(&table).value("id", 1);
    api.conn().insert(insert.into()).await?;

    // SELECT 1 = ALL(SELECT "Test".id FROM "Test");
    let val: Expression<'_> = Value::from(1).into();
    let expr: Expression<'_> = Select::from_table(&table).value(col!("id")).into();
    let expr: Expression<'_> = val.equals(expr.all()).into();
    let query = Select::from_table(&table).value(expr.alias("all"));

    let res = api.conn().select(query.clone()).await?.into_single()?;
    assert_eq!(res["all"], Value::from(true));

    let insert = Insert::single_into(&table).value("id", 2);
    api.conn().insert(insert.into()).await?;

    let res = api.conn().select(query.clone()).await?.into_single()?;
    assert_eq!(res["all"], Value::from(false));

    Ok(())
}

#[cfg(feature = "postgresql")]
#[test_each_connector(tags("postgresql"))]
async fn any_in_expression(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int").await?;

    let insert = Insert::single_into(&table).value("id", 1);
    api.conn().insert(insert.into()).await?;

    // SELECT 1 = ANY(SELECT "Test".id FROM "Test");
    let val: Expression<'_> = Value::from(1).into();
    let expr: Expression<'_> = Select::from_table(&table).value(col!("id")).into();
    let expr: Expression<'_> = val.equals(expr.any()).into();
    let query = Select::from_table(&table).value(expr.alias("any"));

    let res = api.conn().select(query.clone()).await?.into_single()?;
    assert_eq!(res["any"], Value::from(true));

    let insert = Insert::single_into(&table).value("id", 2);
    api.conn().insert(insert.into()).await?;

    let res = api.conn().select(query.clone()).await?.into_single()?;
    assert_eq!(res["any"], Value::from(true));

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_unquote_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api.create_temp_table(&format!("json {json_type}")).await?;

    let insert = Insert::multi_into(&table, vec!["json"])
        .values(vec![serde_json::json!("a")])
        .values(vec![serde_json::json!(1)])
        .values(vec![serde_json::json!({"a":"b"})])
        .values(vec![serde_json::json!(["a", 1])]);

    api.conn().insert(insert.into()).await?;

    let expr: Expression<'_> = json_unquote(col!("json")).into();
    let query = Select::from_table(&table).value(expr.alias("unquote"));

    let res = api.conn().select(query.clone()).await?;

    assert_eq!(res.get(0).unwrap()["unquote"], Value::text("a"));
    assert_eq!(res.get(1).unwrap()["unquote"], Value::text("1"));
    if api.connector_tag().intersects(Tags::MYSQL_MARIADB) {
        assert_eq!(res.get(2).unwrap()["unquote"], Value::text("{\"a\":\"b\"}"));
    } else {
        assert_eq!(res.get(2).unwrap()["unquote"], Value::text("{\"a\": \"b\"}"));
    }
    if api.connector_tag().intersects(Tags::MYSQL_MARIADB) {
        assert_eq!(res.get(3).unwrap()["unquote"], Value::text("[\"a\",1]"));
    } else {
        assert_eq!(res.get(3).unwrap()["unquote"], Value::text("[\"a\", 1]"));
    }

    Ok(())
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_col_equal_json_col(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_temp_table(&format!(
            "{}, json_1 {}, json_2 {}",
            api.autogen_id("id"),
            json_type,
            json_type
        ))
        .await?;

    let insert = Insert::multi_into(&table, vec!["id", "json_1", "json_2"])
        .values(vec![
            Value::from(1),
            serde_json::json!({"a":"b"}).into(),
            serde_json::json!({"a":"b"}).into(),
        ])
        .values(vec![
            Value::from(2),
            serde_json::json!({"a":{"b":"c"}}).into(),
            serde_json::json!("c").into(),
        ]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(&table)
        .column("id")
        .so_that(col!("json_1").equals(col!("json_2")));
    let mut res = api.conn().select(query.clone()).await?.into_iter();

    assert_eq!(res.next().unwrap()["id"], Value::int32(1));
    assert_eq!(res.next(), None);

    let path = match api.system() {
        #[cfg(feature = "postgresql")]
        "postgres" => JsonPath::array(["a", "b"]),
        #[cfg(feature = "mysql")]
        "mysql" => JsonPath::string("$.a.b"),
        _ => unreachable!(),
    };

    // Ensures that using JSON_EXTRACT(`json_col`) = `json_col` works to prevents regressions on MySQL flavoured connectors.
    let expr: Expression<'_> = json_extract(col!("json_1"), path, false).into();
    let query = Select::from_table(&table)
        .column("id")
        .so_that(expr.equals(col!("json_2")));

    let mut res = api.conn().select(query.clone()).await?.into_iter();

    assert_eq!(res.next().unwrap()["id"], Value::int32(2));
    assert_eq!(res.next(), None);

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn update_with_subselect_using_main_table_does_not_throw_error(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_1 = api.create_table("id int, id2 int, val int").await?;
    let table_2 = api.create_table("id int").await?;

    let insert = Insert::single_into(&table_1)
        .value("id", 1)
        .value("id2", 1)
        .value("val", 1);
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_1)
        .value("id", 2)
        .value("id2", 3)
        .value("val", 1);
    api.conn().insert(insert.into()).await?;

    let insert = Insert::single_into(&table_2).value("id", 1);
    api.conn().insert(insert.into()).await?;

    let join = table_2
        .clone()
        .alias("j")
        .on(("j", "id").equals(Column::from(("t1", "id2"))));
    let t1_alias = table_1.clone().alias("t1");
    let selection = Select::from_table(t1_alias).column(("t1", "id")).inner_join(join);

    let id1 = Column::from((&table_1, "id"));
    let conditions = Row::from(vec![id1]).in_selection(selection);
    let update = Update::table(&table_1).set("val", 2).so_that(conditions);

    let res = api.conn().update(update).await;

    api.delete_table(&table_1).await?;
    api.delete_table(&table_2).await?;

    assert_eq!(res?, 1);

    Ok(())
}

#[test_each_connector(tags("mssql"))]
async fn double_rollback_error(api: &mut dyn TestApi) -> crate::Result<()> {
    api.conn().raw_cmd("BEGIN TRAN").await?;
    api.conn().raw_cmd("ROLLBACK").await?;

    let err = api.conn().raw_cmd("ROLLBACK").await.unwrap_err();

    assert!(matches!(err.kind(), ErrorKind::TransactionAlreadyClosed(_)));

    Ok(())
}

#[test_each_connector(tags("mssql"))]
async fn double_commit_error(api: &mut dyn TestApi) -> crate::Result<()> {
    api.conn().raw_cmd("BEGIN TRAN").await?;
    api.conn().raw_cmd("COMMIT").await?;

    let err = api.conn().raw_cmd("COMMIT").await.unwrap_err();

    assert!(matches!(err.kind(), ErrorKind::TransactionAlreadyClosed(_)));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
async fn overflowing_int_errors_out(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("smallint int2, int int4, oid oid").await?;

    let insert = Insert::single_into(&table).value("smallint", (i16::MAX as i64) + 1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '32768' into an INT2 (16-bit signed integer)."));

    let insert = Insert::single_into(&table).value("smallint", (i16::MIN as i64) - 1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '-32769' into an INT2 (16-bit signed integer)."));

    let insert = Insert::single_into(&table).value("int", (i32::MAX as i64) + 1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '2147483648' into an INT4 (32-bit signed integer)."));

    let insert = Insert::single_into(&table).value("int", (i32::MIN as i64) - 1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '-2147483649' into an INT4 (32-bit signed integer)."));

    let insert = Insert::single_into(&table).value("oid", (u32::MAX as i64) + 1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '4294967296' into an OID (32-bit unsigned integer)."));

    let insert = Insert::single_into(&table).value("oid", -1);
    let err = api.conn().insert(insert.into()).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("Unable to fit integer value '-1' into an OID (32-bit unsigned integer)."));

    Ok(())
}
