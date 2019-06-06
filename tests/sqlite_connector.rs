use prisma_query::{connector::Sqlite, Connectional, Transactional};

#[test]
fn should_provide_a_database_connection() {
    let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

    connector
        .with_connection("TEST", |connection| {
            let res = connection.query_raw("SELECT * FROM sqlite_master", &[])?;

            // No results expected.
            assert_eq!(res.into_iter().next().is_none(), true);

            Ok(())
        })
        .unwrap()
}

#[test]
fn should_provide_a_database_transaction() {
    let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

    connector
        .with_transaction("TEST", |transaction| {
            let res = transaction.query_raw("SELECT * FROM sqlite_master", &[])?;

            // No results expected.
            assert_eq!(res.into_iter().next().is_none(), true);

            Ok(())
        })
        .unwrap()
}

const TABLE_DEF: &str = r#"
CREATE TABLE USER (
    ID INT PRIMARY KEY     NOT NULL,
    NAME           TEXT    NOT NULL,
    AGE            INT     NOT NULL,
    SALARY         REAL
);
"#;

const CREATE_USER: &str = r#"
INSERT INTO USER (ID,NAME,AGE,SALARY)
VALUES (1, 'Joe', 27, 20000.00 );
"#;

#[test]
fn should_map_columns_correctly() {
    let connector = Sqlite::new(String::from("db"), 1, true).unwrap();

    connector
        .with_connection("TEST", |connection| {
            connection.query_raw(TABLE_DEF, &[])?;
            connection.query_raw(CREATE_USER, &[])?;

            let res = connection.query_raw("SELECT * FROM USER", &[])?;

            let mut result_count: u32 = 0;

            // Exactly one result expected.
            for row in &res {
                assert_eq!(row.get_as_integer("ID")?, 1);
                assert_eq!(row.get_as_string("NAME")?, "Joe");
                assert_eq!(row.get_as_integer("AGE")?, 27);
                assert_eq!(row.get_as_real("SALARY")?, 20000.0);
                result_count = result_count + 1;
            }

            assert_eq!(result_count, 1);

            Ok(())
        })
        .unwrap()
}
