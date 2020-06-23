use crate::{test_harness::*, BarrelMigrationExecutor};
use barrel::types;

#[test_each_connector(tags("mysql_5_6", "mariadb"))]
async fn database_description_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name()).await;
    let result = dbg!(api.get_database_description().await);
    assert_eq!(result, "{\"tables\":[{\"name\":\"Blog\",\"columns\":[{\"name\":\"id\",\"tpe\":{\"dataType\":\"int\",\"fullDataType\":\"int(11)\",\"characterMaximumLength\":null,\"family\":\"int\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":true},{\"name\":\"string\",\"tpe\":{\"dataType\":\"text\",\"fullDataType\":\"text\",\"characterMaximumLength\":65535,\"family\":\"string\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":false}],\"indices\":[],\"primaryKey\":{\"columns\":[\"id\"],\"sequence\":null},\"foreignKeys\":[]}],\"enums\":[],\"sequences\":[]}".to_string());
}

#[test_each_connector(tags("mysql_8"))]
async fn database_description_for_mysql_8_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name()).await;
    let result = dbg!(api.get_database_description().await);
    assert_eq!(result, "{\"tables\":[{\"name\":\"Blog\",\"columns\":[{\"name\":\"id\",\"tpe\":{\"dataType\":\"int\",\"fullDataType\":\"int\",\"characterMaximumLength\":null,\"family\":\"int\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":true},{\"name\":\"string\",\"tpe\":{\"dataType\":\"text\",\"fullDataType\":\"text\",\"characterMaximumLength\":65535,\"family\":\"string\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":false}],\"indices\":[],\"primaryKey\":{\"columns\":[\"id\"],\"sequence\":null},\"foreignKeys\":[]}],\"enums\":[],\"sequences\":[]}".to_string());
}

#[test_each_connector(tags("postgres"))]
async fn database_description_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name()).await;
    let result = dbg!(api.get_database_description().await);
    assert_eq!(result, "{\"tables\":[{\"name\":\"Blog\",\"columns\":[{\"name\":\"id\",\"tpe\":{\"dataType\":\"integer\",\"fullDataType\":\"int4\",\"characterMaximumLength\":null,\"family\":\"int\",\"arity\":\"required\"},\"default\":{\"SEQUENCE\":\"nextval(\'\\\"Blog_id_seq\\\"\'::regclass)\"},\"autoIncrement\":true},{\"name\":\"string\",\"tpe\":{\"dataType\":\"text\",\"fullDataType\":\"text\",\"characterMaximumLength\":null,\"family\":\"string\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":false}],\"indices\":[],\"primaryKey\":{\"columns\":[\"id\"],\"sequence\":{\"name\":\"Blog_id_seq\",\"initialValue\":1,\"allocationSize\":1}},\"foreignKeys\":[]}],\"enums\":[],\"sequences\":[{\"name\":\"Blog_id_seq\",\"initialValue\":1,\"allocationSize\":1}]}".to_string());
}

#[test_each_connector(tags("sqlite"))]
async fn database_description_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name()).await;
    let result = dbg!(api.get_database_description().await);
    assert_eq!(result, "{\"tables\":[{\"name\":\"Blog\",\"columns\":[{\"name\":\"id\",\"tpe\":{\"dataType\":\"INTEGER\",\"fullDataType\":\"INTEGER\",\"characterMaximumLength\":null,\"family\":\"int\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":true},{\"name\":\"string\",\"tpe\":{\"dataType\":\"TEXT\",\"fullDataType\":\"TEXT\",\"characterMaximumLength\":null,\"family\":\"string\",\"arity\":\"required\"},\"default\":null,\"autoIncrement\":false}],\"indices\":[],\"primaryKey\":{\"columns\":[\"id\"],\"sequence\":null},\"foreignKeys\":[]}],\"enums\":[],\"sequences\":[]}".to_string());
}

async fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) {
    barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("string", types::text());
                });
            },
            db_name,
        )
        .await;
}
