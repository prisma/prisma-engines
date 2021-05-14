mod error;

use super::test_api::*;
#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
use crate::ast::JsonPath;
use crate::{
    connector::{Queryable, TransactionCapable},
    prelude::*,
};
use test_macros::test_each_connector;

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
    let query = Select::default().value(val!(Value::Integer(None)).alias("test"));

    let res = api.conn().select(query).await?;
    let row = res.get(0).unwrap();

    // No results expected.
    assert!(row["test"].is_null());

    Ok(())
}

#[test_each_connector]
async fn select_star_from(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, value int").await?;

    let insert = Insert::single_into(&table).value("value", 3).value("id", 4);
    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::integer(4), row["id"]);
    assert_eq!(Value::integer(3), row["value"]);

    Ok(())
}

#[test_each_connector]
async fn transactions(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("value int").await?;

    let tx = api.conn().start_transaction().await?;
    let insert = Insert::single_into(&table).value("value", 10);

    let rows_affected = tx.execute(insert.into()).await?;
    assert_eq!(1, rows_affected);

    let select = Select::from_table(&table).column("value");
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::integer(10), res[0]);

    tx.rollback().await?;

    let select = Select::from_table(&table).column("value");
    let res = api.conn().select(select).await?;

    assert_eq!(0, res.len());

    Ok(())
}

#[test_each_connector]
async fn in_values_singular(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("id".in_selection(vec![1, 3]));

    let res = api.conn().select(query).await?;
    assert_eq!(2, res.len());

    let row1 = res.get(0).unwrap();
    assert_eq!(Some(1), row1["id"].as_i64());
    assert_eq!(Some(2), row1["id2"].as_i64());

    let row2 = res.get(1).unwrap();
    assert_eq!(Some(3), row2["id"].as_i64());
    assert_eq!(Some(4), row2["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn not_in_values_singular(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![1, 2])
        .values(vec![3, 4])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("id".not_in_selection(vec![1, 3]));

    let res = api.conn().select(query).await?;
    assert_eq!(1, res.len());

    let row1 = res.get(0).unwrap();
    assert_eq!(Some(5), row1["id"].as_i64());
    assert_eq!(Some(6), row1["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn in_values_tuple(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

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
    assert_eq!(Some(1), row1["id"].as_i64());
    assert_eq!(Some(2), row1["id2"].as_i64());

    let row2 = res.get(1).unwrap();
    assert_eq!(Some(3), row2["id"].as_i64());
    assert_eq!(Some(4), row2["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn not_in_values_tuple(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

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
    assert_eq!(Some(5), row["id"].as_i64());
    assert_eq!(Some(6), row["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn order_by_ascend(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![3, 4])
        .values(vec![1, 2])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id2".ascend());

    let res = api.conn().select(query).await?;
    assert_eq!(3, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some(2), row["id2"].as_i64());

    let row = res.get(1).unwrap();
    assert_eq!(Some(3), row["id"].as_i64());
    assert_eq!(Some(4), row["id2"].as_i64());

    let row = res.get(2).unwrap();
    assert_eq!(Some(5), row["id"].as_i64());
    assert_eq!(Some(6), row["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn order_by_descend(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, id2 int").await?;

    let insert = Insert::multi_into(&table, vec!["id", "id2"])
        .values(vec![3, 4])
        .values(vec![1, 2])
        .values(vec![5, 6]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).order_by("id2".descend());

    let res = api.conn().select(query).await?;
    assert_eq!(3, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(5), row["id"].as_i64());
    assert_eq!(Some(6), row["id2"].as_i64());

    let row = res.get(1).unwrap();
    assert_eq!(Some(3), row["id"].as_i64());
    assert_eq!(Some(4), row["id2"].as_i64());

    let row = res.get(2).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some(2), row["id2"].as_i64());

    Ok(())
}

#[test_each_connector]
async fn where_equals(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")]);

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
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("name".like("auk"));
    let res = api.conn().select(query).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn where_not_like(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")]);

    api.conn().insert(insert.into()).await?;

    let query = Select::from_table(table).so_that("name".not_like("auk"));
    let res = api.conn().select(query).await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn inner_join(api: &mut dyn TestApi) -> crate::Result<()> {
    let table1 = api.create_table("id int, name varchar(255)").await?;
    let table2 = api.create_table("t1_id int, is_cat int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"])
        .values(vec![Value::integer(1), Value::integer(1)])
        .values(vec![Value::integer(2), Value::integer(0)]);

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
    let table1 = api.create_table("id int, name varchar(255)").await?;
    let table2 = api.create_table("t1_id int, is_cat int").await?;
    let table3 = api.create_table("id int, foo int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table2, vec!["t1_id", "is_cat"])
        .values(vec![Value::integer(1), Value::integer(1)])
        .values(vec![Value::integer(2), Value::integer(0)]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table3, vec!["id", "foo"]).values(vec![Value::integer(1), Value::integer(1)]);

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
    let table1 = api.create_table("id int, name varchar(255)").await?;
    let table2 = api.create_table("t1_id int, is_cat int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert =
        Insert::multi_into(&table2, vec!["t1_id", "is_cat"]).values(vec![Value::integer(1), Value::integer(1)]);

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
    let table1 = api.create_table("id int, name varchar(255)").await?;
    let table2 = api.create_table("t1_id int, is_cat int").await?;
    let table3 = api.create_table("id int, foo int").await?;

    let insert = Insert::multi_into(&table1, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Belka")]);

    api.conn().insert(insert.into()).await?;

    let insert =
        Insert::multi_into(&table2, vec!["t1_id", "is_cat"]).values(vec![Value::integer(1), Value::integer(1)]);

    api.conn().insert(insert.into()).await?;

    let insert = Insert::multi_into(&table3, vec!["id", "foo"]).values(vec![Value::integer(1), Value::integer(1)]);

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

    println!("{:?}", &res);

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
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")]);

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
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")]);

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
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")])
        .values(vec![Value::integer(3), Value::text("Belka")]);

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
    let table = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::multi_into(&table, vec!["id", "name"])
        .values(vec![Value::integer(1), Value::text("Musti")])
        .values(vec![Value::integer(2), Value::text("Naukio")])
        .values(vec![Value::integer(3), Value::text("Belka")]);

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
        .create_table("id int default 1, name varchar(255) default 'Musti'")
        .await?;

    let changes = api.conn().execute(Insert::single_into(&table).into()).await?;
    assert_eq!(1, changes);

    let select = Select::from_table(&table);

    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[cfg(any(feature = "mssql", feature = "postgresql", feature = "sqlite"))]
#[test_each_connector(tags("mssql", "postgresql", "sqlite"))]
async fn returning_insert(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.get_name();

    api.conn()
        .raw_cmd(&format!(
            "CREATE TABLE {} (id int primary key, name varchar(255))",
            table
        ))
        .await?;

    let insert = Insert::single_into(&table).value("id", 1).value("name", "Naukio");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "name"]))
        .await;

    api.conn().raw_cmd(&format!("DROP TABLE {}", table)).await?;

    let res = res?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[cfg(all(feature = "mssql", feature = "bigdecimal"))]
#[test_each_connector(tags("mssql"))]
async fn returning_decimal_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let dec = BigDecimal::from_str("17661757261711787211853")?;
    let table = api.create_table("id int, val numeric(26,0)").await?;
    let col = Column::from("val").type_family(TypeFamily::Decimal(Some((26, 0))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, dec.clone());

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some(&dec), row["val"].as_numeric());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_constant_nvarchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, val nvarchar(4000)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Constant(4000))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_max_nvarchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, val nvarchar(max)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Maximum)));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_constant_varchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, val varchar(4000)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Constant(4000))));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("meowmeow"), row["val"].as_str());

    Ok(())
}

#[cfg(feature = "mssql")]
#[test_each_connector(tags("mssql"))]
async fn returning_max_varchar_insert_with_type_defs(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, val varchar(max)").await?;
    let col = Column::from("val").type_family(TypeFamily::Text(Some(TypeDataLength::Maximum)));

    let insert = Insert::single_into(&table).value("id", 2).value(col, "meowmeow");

    let res = api
        .conn()
        .insert(Insert::from(insert).returning(vec!["id", "val"]))
        .await?;

    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
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

    assert_eq!(Some(&Value::Integer(Some(1))), row.get("foo"));
    assert_eq!(Some(&Value::Integer(Some(2))), row.get("bar"));

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_table(&format!("id int, name varchar(255), {}", constraint))
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_table(&format!("id int default 10, name varchar(255), {}", constraint))
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
    assert_eq!(Some(10), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_single_unique_with_autogen_default(
    api: &mut dyn TestApi,
) -> crate::Result<()> {
    let table_name = api
        .create_table(&format!("{}, name varchar(255)", api.autogen_id("id")))
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[cfg(any(feature = "mssql", feature = "postgresql"))]
#[test_each_connector(tags("postgresql", "mssql"))]
async fn single_insert_conflict_do_nothing_with_returning(api: &mut dyn TestApi) -> crate::Result<()> {
    let constraint = api.unique_constraint("id");

    let table_name = api
        .create_table(&format!("id int, name varchar(255), {}", constraint))
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
        .create_table(&format!(
            "id int, name varchar(255), {}, {}",
            id_constraint, name_constraint
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("Belka"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_two_uniques_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let id_constraint = api.unique_constraint("id");
    let name_constraint = api.unique_constraint("name");

    let table_name = api
        .create_table(&format!(
            "id int, name varchar(255) default 'Musti', {}, {}",
            id_constraint, name_constraint
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compoud_unique(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_table("id int, name varchar(255)").await?;
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compoud_unique_with_default(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_table("id int, name varchar(255) default 'Musti'").await?;
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_unique_with_autogen(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api
        .create_table(&format!("{}, name varchar(100)", api.autogen_id("id")))
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn single_insert_conflict_do_nothing_compoud_unique_with_autogen_default(
    api: &mut dyn TestApi,
) -> crate::Result<()> {
    let table_name = api
        .create_table(&format!("{}, name varchar(100) default 'Musti'", api.autogen_id("id")))
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
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    let row = res.get(1).unwrap();
    assert_eq!(Some(2), row["id"].as_i64());
    assert_eq!(Some("Musti"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn updates(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_table("id int, name varchar(255)").await?;

    let insert = Insert::single_into(&table_name).value("name", "Musti").value("id", 1);
    api.conn().insert(insert.into()).await?;

    let update = Update::table(&table_name).set("name", "Naukio").so_that("id".equals(1));
    let changes = api.conn().execute(update.into()).await?;

    assert_eq!(1, changes);

    let select = Select::from_table(&table_name).order_by("id".ascend());
    let res = api.conn().select(select).await?;
    assert_eq!(1, res.len());

    let row = res.get(0).unwrap();
    assert_eq!(Some(1), row["id"].as_i64());
    assert_eq!(Some("Naukio"), row["name"].as_str());

    Ok(())
}

#[test_each_connector]
async fn deletes(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_name = api.create_table("id int, name varchar(255)").await?;

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

#[test_each_connector(tags("mysql"))]
async fn text_columns_with_non_utf8_encodings_can_be_queried(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_table("id integer auto_increment primary key, value varchar(100) character set gb18030")
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

#[test_each_connector(tags("mysql"))]
async fn filtering_by_json_values_does_not_work_but_does_not_crash(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_table("id int4 auto_increment primary key, nested json not null")
        .await?;

    let insert = Insert::multi_into(&table, &["nested"])
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
        .create_table("id int4 auto_increment primary key, f float not null")
        .await?;

    let insert = Insert::single_into(&table).value("f", 6.4123456);
    api.conn().query(insert.into()).await?;

    let select = Select::from_table(&table).column("f");
    let row = api.conn().query(select.into()).await?.into_single()?;
    let value = row.at(0).unwrap();

    assert_eq!(Some(6.4123454), value.as_f32());

    Ok(())
}

#[test_each_connector(tags("mysql"))]
#[cfg(feature = "bigdecimal")]
async fn newdecimal_conversion_is_handled_correctly(api: &mut dyn TestApi) -> crate::Result<()> {
    let select = Select::default().value(sum(Value::integer(1)).alias("theone"));
    let result = api.conn().select(select).await?;

    assert_eq!(
        Value::Numeric(Some("1.0".parse().unwrap())),
        result.into_single().unwrap()[0]
    );

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn unsigned_integers_are_handled(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_table("id int4 auto_increment primary key, big bigint unsigned")
        .await?;

    let insert = Insert::multi_into(&table, &["big"])
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

#[cfg(feature = "json")]
#[test_each_connector(tags("mysql", "postgresql"))]
async fn json_filtering_works(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };

    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
        assert_eq!(Some(2), row["id"].as_i64());
    }

    // Not equals
    {
        let select =
            Select::from_table(&table).so_that(Column::from("obj").not_equals(serde_json::json!({ "a": "a" })));

        let result = api.conn().query(select.into()).await?;

        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(2), row["id"].as_i64());
    }

    Ok(())
}

#[test_each_connector(tags("mssql", "postgresql"))]
async fn xml_filtering_works(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_table(&format!("{}, xmlfield {}", api.autogen_id("id"), "xml"))
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
        assert_eq!(Some(2), row["id"].as_i64());
    }

    // Not equals
    {
        let select =
            Select::from_table(&table).so_that(Column::from("xmlfield").not_equals(Value::xml("<horse>neigh</horse>")));
        let result = api.conn().query(select.into()).await?;
        assert_eq!(result.len(), 1);

        let row = result.into_single()?;
        assert_eq!(Some(1), row["id"].as_i64());
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
    let table = api.create_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 1).value("b", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") + col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(3), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_add_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 3).value("c", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") + val!(col!("b") + col!("c")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(7), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(1), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_three_items(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 1).value("c", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b") - col!("c"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(0), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_sub_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int, c int").await?;

    let insert = Insert::single_into(&table).value("a", 2).value("b", 3).value("c", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - val!(col!("b") + col!("c")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(-2), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_mul_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int").await?;

    let insert = Insert::single_into(&table).value("a", 6);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") * col!("a"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(36), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_mul_two_levels(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 6).value("b", 1);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") * (col!("a") - col!("b")));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(30), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_multiple_operations(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a int, b int").await?;

    let insert = Insert::single_into(&table).value("a", 4).value("b", 2);
    api.conn().insert(insert.into()).await?;

    let q = Select::from_table(&table).value(col!("a") - col!("b") * col!("b"));
    let row = api.conn().select(q).await?.into_single()?;

    assert_eq!(Some(0), row[0].as_i64());

    Ok(())
}

#[test_each_connector]
async fn op_test_div_one_level(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("a real, b real").await?;

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
        .create_table(&format!("id SERIAL PRIMARY KEY, value {}", &type_name))
        .await?;

    api.conn()
        .insert(Insert::single_into(&table).value("value", "A").into())
        .await?;

    api.conn()
        .insert(Insert::single_into(&table).value("value", "B").into())
        .await?;

    api.conn()
        .insert(Insert::single_into(&table).value("value", Value::Enum(None)).into())
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".ascend());
    let res = api.conn().select(select).await?;

    let row = res.get(0).unwrap();
    assert_eq!(Some(&Value::enum_variant("A")), row.at(0));

    let row = res.get(1).unwrap();
    assert_eq!(Some(&Value::enum_variant("B")), row.at(0));

    let row = res.get(2).unwrap();
    assert_eq!(Some(&Value::Enum(None)), row.at(0));

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
#[cfg(all(feature = "json", feature = "postgresql"))]
async fn row_to_json_normal(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte = Select::default()
        .value(val!("hello_world").alias("toto"))
        .into_cte("one");
    let select = Select::from_table("one").value(row_to_json("one", false)).with(cte);
    let result = api.conn().select(select).await?;

    assert_eq!(
        Value::Json(Some(serde_json::json!({
            "toto": "hello_world"
        }))),
        result.into_single().unwrap()[0]
    );

    Ok(())
}

#[test_each_connector(tags("postgresql"))]
#[cfg(all(feature = "json", feature = "postgresql"))]
async fn row_to_json_pretty(api: &mut dyn TestApi) -> crate::Result<()> {
    let cte = Select::default()
        .value(val!("hello_world").alias("toto"))
        .into_cte("one");
    let select = Select::from_table("one").value(row_to_json("one", true)).with(cte);
    let result = api.conn().select(select).await?;

    assert_eq!(
        Value::Json(Some(serde_json::json!({
            "toto": "hello_world"
        }))),
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

    if api.system() == "postgres" {
        assert_eq!(Some(&Value::text("1")), row.at(0));
    } else {
        assert_eq!(Some(&Value::integer(1)), row.at(0));
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

    if api.system() == "postgres" {
        assert_eq!(Some(&Value::text("1")), row.at(0));
        assert_eq!(Some(&Value::text("2")), row.at(1));
    } else {
        assert_eq!(Some(&Value::integer(1)), row.at(0));
        assert_eq!(Some(&Value::integer(2)), row.at(1));
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
    let table = api.create_table("id1 varchar(3), id2 varchar(3)").await?;

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
    let table = api.create_table("id1 varchar(3), id2 varchar(3)").await?;

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
    let table_1 = api.create_table("id1 int, id2 int, data varchar(3)").await?;
    let table_2 = api.create_table("id3 int, id4 int").await?;

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

    assert_eq!(Some(&Value::integer(1)), row.at(0));
    assert_eq!(Some(&Value::integer(2)), row.at(1));

    Ok(())
}

#[test_each_connector]
async fn join_with_non_matching_compound_columns(api: &mut dyn TestApi) -> crate::Result<()> {
    let table_1 = api.create_table("id1 int, id2 int, data varchar(3)").await?;
    let table_2 = api.create_table("id3 int, id4 int").await?;

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

    assert_eq!(Some(&Value::integer(2)), row.at(0));
    assert_eq!(Some(&Value::integer(3)), row.at(1));

    Ok(())
}

#[test_each_connector(ignore("sqlite"))]
async fn insert_default_keyword(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table("id int, value int DEFAULT 1").await?;

    let insert = Insert::single_into(&table)
        .value("value", default_value())
        .value("id", 4);

    api.conn().execute(insert.into()).await?;

    let select = Select::from_table(&table);
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Value::integer(4), row["id"]);
    assert_eq!(Value::integer(1), row["value"]);

    Ok(())
}

#[cfg(feature = "bigdecimal")]
#[test_each_connector(tags("postgresql"))]
async fn ints_read_write_to_numeric(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let table = api.create_table("id int, value numeric(12,2)").await?;

    let insert = Insert::multi_into(&table, &["id", "value"])
        .values(vec![Value::integer(1), Value::double(1234.5)])
        .values(vec![Value::integer(2), Value::integer(1234)])
        .values(vec![Value::integer(3), Value::integer(12345)]);

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

#[cfg(feature = "bigdecimal")]
#[test_each_connector(tags("postgresql"))]
async fn bigdecimal_read_write_to_floating(api: &mut dyn TestApi) -> crate::Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let table = api.create_table("id int, a float4, b float8").await?;
    let val = BigDecimal::from_str("0.1").unwrap();

    let insert = Insert::multi_into(&table, &["id", "a", "b"]).values(vec![
        Value::integer(1),
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
    let exprs: Vec<Expression> = vec![Value::Text(None).into(), Value::text("Individual").into()];
    let select = Select::default().value(coalesce(exprs).alias("val"));
    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some("Individual"), row["val"].as_str());

    Ok(())
}

#[cfg(all(feature = "json", feature = "mysql"))]
#[test_each_connector(tags("mysql"))]
async fn json_extract_path_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_table(&format!("{}, obj json", api.autogen_id("id"))).await?;

    let insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": "c" } }));
    let second_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a": { "b": [1, 2, 3] } }));
    let third_insert = Insert::single_into(&table).value("obj", serde_json::json!({ "a\":{": "b" }));

    api.conn().insert(insert.into()).await?;
    api.conn().insert(second_insert.into()).await?;
    api.conn().insert(third_insert.into()).await?;

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.a.b"), false).into();
    let select = Select::from_table(&table).so_that(extract.equals("c"));
    let row = api.conn().select(select).await?.into_single()?;

    // Test object extraction
    assert_eq!(Some(&serde_json::json!({ "a": { "b": "c" } })), row["obj"].as_json());

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.a.b[1]"), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(2));
    let row = api.conn().select(select).await?.into_single()?;

    // Test array index extraction
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        row["obj"].as_json()
    );

    let extract: Expression = json_extract(col!("obj"), JsonPath::string("$.\"a\\\":{\""), false).into();
    let select = Select::from_table(&table).so_that(extract.equals("b"));
    let row = api.conn().select(select).await?.into_single()?;

    // Test escaped chars in keys
    assert_eq!(Some(&serde_json::json!({ "a\":{": "b" })), row["obj"].as_json());

    Ok(())
}

#[cfg(all(feature = "json", feature = "postgresql"))]
#[test_each_connector(tags("postgresql"))]
async fn json_extract_array_path_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_table(&format!("{}, obj jsonb", api.autogen_id("id")))
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
    assert_eq!(Some(&serde_json::json!({ "a": { "b": "c" } })), row["obj"].as_json());

    // Test equality with Json value
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals(serde_json::Value::String("c".to_owned())));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(Some(&serde_json::json!({ "a": { "b": "c" } })), row["obj"].as_json());

    // Test array index extraction
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a", "b", "1"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals("2"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        row["obj"].as_json()
    );

    // Test escaped chars in keys
    let extract: Expression = json_extract(col!("obj"), JsonPath::array(["a\":{"]), false).into();
    let select = Select::from_table(&table).so_that(extract.equals("\"b\""));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(Some(&serde_json::json!({ "a\":{": "b" })), row["obj"].as_json());

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_contains_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains("[2]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        row["obj"].as_json()
    );

    // Assert contains string
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains("[\"bar\"]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        row["obj"].as_json()
    );

    // Assert contains object
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains("[{\"bar\": \"foo\"}]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        row["obj"].as_json()
    );

    // Assert contains array
    let select = Select::from_table(&table).so_that(path.clone().json_array_contains("[[1, 2]]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
        row["obj"].as_json()
    );

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_not_contains_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    assert_eq!(Some(&serde_json::json!({ "a": { "b": [4, 5] } })), row["obj"].as_json());

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_begins_with_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with("1"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        row["obj"].as_json()
    );

    // Assert starts with string
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with("\"foo\""));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        row["obj"].as_json()
    );

    // Assert starts with object
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with("{\"foo\": \"bar\"}"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        row["obj"].as_json()
    );

    // Assert starts with array
    let select = Select::from_table(&table).so_that(path.clone().json_array_begins_with("[1, 2]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
        row["obj"].as_json()
    );

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_not_begins_with_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    let select = Select::from_table(&table).so_that(path.clone().json_array_not_begins_with("1"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(Some(&serde_json::json!({ "a": { "b": [4, 5] } })), row["obj"].as_json());

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_ends_into_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into("3"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [1, 2, 3] } })),
        row["obj"].as_json()
    );

    // Assert ends with string
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into("\"bar\""));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": ["foo", "bar"] } })),
        row["obj"].as_json()
    );

    // Assert ends with object
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into("{\"bar\": \"foo\"}"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [{ "foo": "bar" }, { "bar": "foo" }] } })),
        row["obj"].as_json()
    );

    // Assert ends with array
    let select = Select::from_table(&table).so_that(path.clone().json_array_ends_into("[3, 4]"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(
        Some(&serde_json::json!({ "a": { "b": [[1, 2], [3, 4]] } })),
        row["obj"].as_json()
    );

    Ok(())
}

#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
#[test_each_connector(tags("postgresql", "mysql"))]
async fn json_array_not_ends_into_fun(api: &mut dyn TestApi) -> crate::Result<()> {
    let json_type = match api.system() {
        "postgres" => "jsonb",
        _ => "json",
    };
    let table = api
        .create_table(&format!("{}, obj {}", api.autogen_id("id"), json_type))
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
    let select = Select::from_table(&table).so_that(path.clone().json_array_not_ends_into("2"));
    let row = api.conn().select(select).await?.into_single()?;
    assert_eq!(Some(&serde_json::json!({ "a": { "b": [4, 5] } })), row["obj"].as_json());

    Ok(())
}
