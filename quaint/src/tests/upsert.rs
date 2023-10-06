use super::test_api::*;
use crate::{connector::Queryable, prelude::*};
use quaint_test_macros::test_each_connector;

#[test_each_connector(tags("postgresql", "sqlite"))]
async fn upsert_on_primary_key(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int primary key, x int").await?;

    let update = Update::table(&table).set("x", 2).so_that((&table, "id").equals(1));

    let insert: Insert = Insert::single_into(&table).value("id", 1).value("x", 1).into();

    let query: Query = insert
        .on_conflict(OnConflict::Update(update, Vec::from(["id".into()])))
        .into();
    // Insert
    let count = api.conn().execute(query.clone()).await?;

    assert_eq!(count, 1);

    let select = Select::from_table(&table);
    let row = api.conn().select(select.clone()).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(1), row["x"].as_i32());

    // // Update
    let count = api.conn().execute(query).await?;
    assert_eq!(count, 1);

    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(2), row["x"].as_i32());

    Ok(())
}

#[test_each_connector(tags("postgresql", "sqlite"))]
async fn upsert_on_unique_field(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_temp_table("id int primary key, x int UNIQUE, y int").await?;

    let update = Update::table(&table).set("y", 2).so_that((&table, "id").equals(1));

    let insert: Insert = Insert::single_into(&table)
        .value("id", 1)
        .value("x", 1)
        .value("y", 1)
        .into();

    let query: Query = insert
        .on_conflict(OnConflict::Update(update, Vec::from(["x".into()])))
        .into();
    // Insert
    let count = api.conn().execute(query.clone()).await?;

    assert_eq!(count, 1);

    let select = Select::from_table(&table);
    let row = api.conn().select(select.clone()).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(1), row["x"].as_i32());
    assert_eq!(Some(1), row["y"].as_i32());

    // Update
    let count = api.conn().execute(query).await?;
    assert_eq!(count, 1);

    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(1), row["x"].as_i32());
    assert_eq!(Some(2), row["y"].as_i32());

    Ok(())
}

#[test_each_connector(tags("postgresql", "sqlite"))]
async fn upsert_on_multiple_unique_fields(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api
        .create_temp_table("id int primary key, x int, y int, CONSTRAINT ux_x_y UNIQUE (x, y)")
        .await?;

    let update = Update::table(&table).set("y", 2).so_that((&table, "id").equals(1));

    let insert: Insert = Insert::single_into(&table)
        .value("id", 1)
        .value("x", 1)
        .value("y", 1)
        .into();

    let query: Query = insert
        .on_conflict(OnConflict::Update(update, Vec::from(["x".into(), "y".into()])))
        .into();

    // Insert
    let count = api.conn().execute(query.clone()).await?;

    assert_eq!(count, 1);

    let select = Select::from_table(&table);
    let row = api.conn().select(select.clone()).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(1), row["x"].as_i32());
    assert_eq!(Some(1), row["y"].as_i32());

    // Update
    let count = api.conn().execute(query).await?;
    assert_eq!(count, 1);

    let row = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some(1), row["id"].as_i32());
    assert_eq!(Some(1), row["x"].as_i32());
    assert_eq!(Some(2), row["y"].as_i32());

    Ok(())
}
