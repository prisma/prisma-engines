use datamodel_connector::{Connector, ReferentialIntegrity};
use enumflags2::BitFlags;

pub trait DatasourceProvider {
    /// Passes the provider arg from the datasource. Must return true for all provider names it can handle.
    fn is_provider(&self, provider: &str) -> bool;

    fn canonical_name(&self) -> &str;

    fn connector(&self) -> Box<dyn Connector>;

    fn allowed_referential_integrity_settings(&self) -> BitFlags<ReferentialIntegrity> {
        use ReferentialIntegrity::*;

        ForeignKeys | Prisma
    }

    fn default_referential_integrity(&self) -> ReferentialIntegrity {
        ReferentialIntegrity::ForeignKeys
    }
}
