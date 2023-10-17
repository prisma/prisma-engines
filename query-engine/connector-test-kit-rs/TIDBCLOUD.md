# How to test TiDB Cloud

1. Use TiDB Cloud connection information to replace the following code in the `query-engine/connector-test-kit-rs//query-tests-setup/src/connector_tag/mod.rs`:

    ```
        ConnectorVersion::MySql(v) => match v {
            Some(MySqlVersion::V5_6) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-6:3306/{database}"),
            Some(MySqlVersion::V5_7) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-7:3306/{database}"),
            Some(MySqlVersion::V8) if is_ci => format!("mysql://root:prisma@test-db-mysql-8:3306/{database}"),
            Some(MySqlVersion::MariaDb) if is_ci => {
                format!("mysql://root:prisma@test-db-mysql-mariadb:3306/{database}")
            }
            Some(MySqlVersion::V5_6) => format!("mysql://root:prisma@127.0.0.1:3309/{database}"),
            Some(MySqlVersion::V5_7) => format!("mysql://root:prisma@127.0.0.1:3306/{database}"),
            Some(MySqlVersion::V8) => format!("mysql://root:prisma@127.0.0.1:3307/{database}"),
            Some(MySqlVersion::MariaDb) => {
                format!("mysql://root:prisma@127.0.0.1:3308/{database}")
            }

            None => unreachable!("A versioned connector must have a concrete version to run."),
        },
    ```

    ```
        ConnectorVersion::MySql(v) => match v {
            Some(MySqlVersion::V5_7) if is_ci => format!("mysql://user:password@host:4000/{database}?sslaccept=strict"),

            Some(MySqlVersion::V5_7) => format!("mysql://user:password@host:4000/{database}?sslaccept=strict"),
    ```

2. `export WORKSPACE_ROOT=/Users/shiyuhang/github/prisma-engines`
3. `make dev-tidbcloud-mysql5_7`
4. Input the TiDB Cloud connection information in the `.test_config` file
5. Run test by `cargo test --package query-engine-tests -- --test-threads=1`