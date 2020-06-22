use super::super::test_api::QueryEngine;
use crate::context::PrismaContext;
use quaint::{prelude::Queryable, single::Quaint};

pub type TestResult = anyhow::Result<()>;

pub struct TestApi {
    connector_name: &'static str,
    provider: &'static str,
    database_string: String,
    connection: Quaint,
}

impl TestApi {
    fn datasource(&self) -> String {
        format!(
            r#"
                datasource my_db {{
                    provider = "{provider}"
                    url = "{url}"
                }}
            "#,
            provider = self.provider,
            url = self.database_string,
        )
    }

    pub async fn execute_sql(&self, sql: &str) -> anyhow::Result<()> {
        self.connection.execute_raw(sql, &[]).await?;

        Ok(())
    }

    pub async fn introspect_and_start_query_engine(&self) -> anyhow::Result<(DatamodelAssertions, QueryEngine)> {
        let datasource = self.datasource();

        let introspection_result = introspection_core::RpcImpl::introspect_internal(datasource, false)
            .await
            .map_err(|err| anyhow::anyhow!("{:?}", err.data))?;

        let dml = datamodel::parse_datamodel(&introspection_result.datamodel).unwrap();
        let config = datamodel::parse_configuration(&introspection_result.datamodel).unwrap();

        let context = PrismaContext::builder(config, dml.clone())
            .enable_raw_queries(true)
            .build()
            .await
            .unwrap();

        Ok((DatamodelAssertions(dml), QueryEngine::new(context)))
    }

    pub fn is_mysql_5_6(&self) -> bool {
        self.connector_name == "mysql_5_6"
    }

    pub fn is_maria_db(&self) -> bool {
        self.connector_name == "mysql_mariadb"
    }
}

#[derive(Debug)]
pub struct DatamodelAssertions(datamodel::Datamodel);

impl DatamodelAssertions {
    pub fn assert_model<F>(self, name: &str, assert_fn: F) -> anyhow::Result<Self>
    where
        F: for<'a> FnOnce(ModelAssertions<'a>) -> anyhow::Result<ModelAssertions<'a>>,
    {
        let model = self
            .0
            .find_model(name)
            .ok_or_else(|| anyhow::anyhow!("Assertion error: could not find model {}", name))?;

        assert_fn(ModelAssertions(model))?;

        Ok(self)
    }
}

pub struct ModelAssertions<'a>(&'a datamodel::dml::Model);

impl<'a> ModelAssertions<'a> {
    pub fn assert_field_type(self, name: &str, r#type: datamodel::dml::ScalarType) -> anyhow::Result<Self> {
        let field = self
            .0
            .find_field(name)
            .ok_or_else(|| anyhow::anyhow!("Assertion error: could not find field {}", name))?;

        anyhow::ensure!(
            field.field_type == datamodel::dml::FieldType::Base(r#type, None),
            "Assertion error: expected the field {} to have type {:?}, but found {:?}",
            field.name,
            r#type,
            &field.field_type,
        );

        Ok(self)
    }

    pub fn assert_field_enum_type(self, name: &str, enum_name: &str) -> anyhow::Result<Self> {
        let field = self
            .0
            .find_field(name)
            .ok_or_else(|| anyhow::anyhow!("Assertion error: could not find field {}", name))?;

        anyhow::ensure!(
            field.field_type == datamodel::dml::FieldType::Enum(enum_name.into()),
            "Assertion error: expected the field {} to have enum type {:?}, but found {:?}",
            field.name,
            enum_name,
            &field.field_type,
        );

        Ok(self)
    }
}

pub async fn mysql_8_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mysql_8_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "mysql_8",
        connection: Quaint::new(&mysql_url).await.unwrap(),
        database_string: mysql_url,
        provider: "mysql",
    }
}

pub async fn mysql_5_6_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mysql_5_6_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "mysql_5_6",
        connection: Quaint::new(&mysql_url).await.unwrap(),
        database_string: mysql_url,
        provider: "mysql",
    }
}

pub async fn mysql_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mysql_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "mysql",
        connection: Quaint::new(&mysql_url).await.unwrap(),
        database_string: mysql_url,
        provider: "mysql",
    }
}

pub async fn mysql_mariadb_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mariadb_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "mysql_mariadb",
        connection: Quaint::new(&mysql_url).await.unwrap(),
        database_string: mysql_url,
        provider: "mysql",
    }
}

pub async fn postgres_test_api(db_name: &str) -> TestApi {
    let postgres_url = test_setup::postgres_10_url(db_name);

    test_setup::create_postgres_database(&postgres_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "postgres",
        connection: Quaint::new(&postgres_url).await.unwrap(),
        database_string: postgres_url,
        provider: "postgres",
    }
}

pub async fn postgres9_test_api(db_name: &str) -> TestApi {
    let postgres_url = test_setup::postgres_9_url(db_name);

    test_setup::create_postgres_database(&postgres_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "postgres9",
        connection: Quaint::new(&postgres_url).await.unwrap(),
        database_string: postgres_url,
        provider: "postgres",
    }
}

pub async fn postgres11_test_api(db_name: &str) -> TestApi {
    let postgres_url = test_setup::postgres_11_url(db_name);

    test_setup::create_postgres_database(&postgres_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "postgres11",
        connection: Quaint::new(&postgres_url).await.unwrap(),
        database_string: postgres_url,
        provider: "postgres",
    }
}

pub async fn postgres12_test_api(db_name: &str) -> TestApi {
    let postgres_url = test_setup::postgres_12_url(db_name);

    test_setup::create_postgres_database(&postgres_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        connector_name: "postgres12",
        connection: Quaint::new(&postgres_url).await.unwrap(),
        database_string: postgres_url,
        provider: "postgres",
    }
}
