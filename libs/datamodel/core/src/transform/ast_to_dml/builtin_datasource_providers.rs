use super::datasource_provider::DatasourceProvider;
use crate::common::provider_names::*;
use datamodel_connector::{Connector, ReferentialIntegrity};
use mongodb_datamodel_connector::MongoDbDatamodelConnector;
use sql_datamodel_connector::SqlDatamodelConnectors;

pub struct SqliteDatasourceProvider {
    referential_integrity: ReferentialIntegrity,
}

impl SqliteDatasourceProvider {
    pub fn new(referential_integrity: ReferentialIntegrity) -> Self {
        Self { referential_integrity }
    }
}

impl DatasourceProvider for SqliteDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == SQLITE_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        SQLITE_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::sqlite(self.referential_integrity))
    }
}

pub struct PostgresDatasourceProvider {
    referential_integrity: ReferentialIntegrity,
}

impl PostgresDatasourceProvider {
    pub fn new(referential_integrity: ReferentialIntegrity) -> Self {
        Self { referential_integrity }
    }
}

impl DatasourceProvider for PostgresDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == POSTGRES_SOURCE_NAME || provider == POSTGRES_SOURCE_NAME_HEROKU
    }

    fn canonical_name(&self) -> &str {
        POSTGRES_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::postgres(self.referential_integrity))
    }
}

pub struct MySqlDatasourceProvider {
    referential_integrity: ReferentialIntegrity,
}

impl MySqlDatasourceProvider {
    pub fn new(referential_integrity: ReferentialIntegrity) -> Self {
        Self { referential_integrity }
    }
}

impl DatasourceProvider for MySqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MYSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MYSQL_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::mysql(self.referential_integrity))
    }
}

pub struct MsSqlDatasourceProvider {
    referential_integrity: ReferentialIntegrity,
}

impl MsSqlDatasourceProvider {
    pub fn new(referential_integrity: ReferentialIntegrity) -> Self {
        Self { referential_integrity }
    }
}

impl DatasourceProvider for MsSqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MSSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MSSQL_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::mssql(self.referential_integrity))
    }
}

pub struct MongoDbDatasourceProvider {}
impl MongoDbDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for MongoDbDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MONGODB_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MONGODB_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(MongoDbDatamodelConnector::new())
    }

    fn default_referential_integrity(&self) -> ReferentialIntegrity {
        ReferentialIntegrity::Prisma
    }

    fn allowed_referential_integrity_settings(&self) -> enumflags2::BitFlags<ReferentialIntegrity> {
        ReferentialIntegrity::Prisma.into()
    }
}
