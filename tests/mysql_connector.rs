use mysql::OptsBuilder;
use prisma_query::{connector::Mysql, Connectional, Transactional};
use std::env;

fn get_config() -> OptsBuilder {
    let mut config = OptsBuilder::new();
    config.ip_or_hostname(env::var("TEST_MYSQL_HOST"));
    config.tcp_por(env::var("TEST_MYSQL_HOST").map(|x| x.parse::<u16>()));
    config.db_name(env::var("TEST_MYSQL_PORT").unwrap());
    config.pass(env::var("TEST_MYSQL_PASSWORD").unwrap());
    config.user(env::var("TEST_MYSQL_USER").unwrap());
    config
}

#[test]
fn should_provide_a_database_connection() {
    let connector = Mysql::new(get_config()).unwrap();

    connector
        .with_connection("TEST", |connection| {
            let res = connection.query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )?;

            // No results expected.
            assert_eq!(res.into_iter().next().is_none(), true);

            Ok(())
        })
        .unwrap()
}

#[test]
fn should_provide_a_database_transaction() {
    let connector = Mysql::new(get_config()).unwrap();

    connector
        .with_transaction("TEST", |transaction| {
            let res = transaction.query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )?;

            // No results expected.
            assert_eq!(res.into_iter().next().is_none(), true);

            Ok(())
        })
        .unwrap()
}

const TABLE_DEF: &str = r#"
CREATE TABLE "user"(
    id       int4    PRIMARY KEY     NOT NULL,
    name     text    NOT NULL,
    age      int4    NOT NULL,
    salary   float4
);
"#;

const CREATE_USER: &str = r#"
INSERT INTO "user" (id, name, age, salary)
VALUES (1, 'Joe', 27, 20000.00 );
"#;

const DROP_TABLE: &str = "DROP TABLE IF EXISTS \"user\";";

#[test]
fn should_map_columns_correctly() {
    let connector = Mysql::new(get_config()).unwrap();

    connector
        .with_connection("TEST", |connection| {
            connection.query_raw(DROP_TABLE, &[]).unwrap();
            connection.query_raw(TABLE_DEF, &[]).unwrap();
            connection.query_raw(CREATE_USER, &[]).unwrap();

            let res = connection.query_raw("SELECT * FROM \"user\"", &[]).unwrap();

            let mut result_count: u32 = 0;

            // Exactly one result expected.
            for row in &res {
                assert_eq!(row.get_as_integer("id")?, 1);
                assert_eq!(row.get_as_string("name")?, "Joe");
                assert_eq!(row.get_as_integer("age")?, 27);
                assert_eq!(row.get_as_real("salary")?, 20000.0);
                result_count = result_count + 1;
            }

            assert_eq!(result_count, 1);

            Ok(())
        })
        .unwrap()
}
