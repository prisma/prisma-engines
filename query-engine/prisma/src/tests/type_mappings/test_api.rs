use super::super::test_api::QueryEngine;
use crate::context::PrismaContext;
use quaint::prelude::Queryable;

pub type TestResult = anyhow::Result<()>;

pub struct TestApi {
    provider: &'static str,
    database_string: String,
    is_pgbouncer: bool,
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

    pub async fn execute(&self, sql: &str) -> anyhow::Result<()> {
        let conn = quaint::single::Quaint::new(&self.database_string).await?;

        conn.execute_raw(sql, &[]).await?;

        Ok(())
    }

    pub async fn create_engine(&self) -> anyhow::Result<(DatamodelAssertions, QueryEngine)> {
        let datasource = self.datasource();

        let schema = introspection_core::RpcImpl::introspect_internal(datasource)
            .await
            .map_err(|err| anyhow::anyhow!("{:?}", err.data))?;

        let context = PrismaContext::builder()
            .enable_raw_queries(true)
            .datamodel(schema.clone())
            .force_transactions(self.is_pgbouncer)
            .build()
            .await
            .unwrap();

        eprintln!("{}", schema);
        let schema = datamodel::parse_datamodel(&schema).unwrap();

        Ok((DatamodelAssertions(schema), QueryEngine::new(context)))
    }
}

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
            field.field_type == datamodel::dml::FieldType::Base(r#type),
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
        database_string: mysql_url,
        provider: "mysql",
        is_pgbouncer: false,
    }
}

pub async fn mysql_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mysql_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        database_string: mysql_url,
        provider: "mysql",
        is_pgbouncer: false,
    }
}

pub async fn mysql_mariadb_test_api(db_name: &str) -> TestApi {
    let mysql_url = test_setup::mariadb_url(db_name);

    test_setup::create_mysql_database(&mysql_url.parse().unwrap())
        .await
        .unwrap();

    TestApi {
        database_string: mysql_url,
        provider: "mysql",
        is_pgbouncer: false,
    }
}
