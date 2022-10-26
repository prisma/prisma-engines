use super::*;
use crate::{SqlDatamodelRenderer};

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::Enums,
    ConnectorCapability::EnumArrayPush,
    ConnectorCapability::Json,
    ConnectorCapability::AutoIncrementAllowedOnNonId,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::CreateMany,
    ConnectorCapability::WritableAutoincField,
    ConnectorCapability::CreateSkipDuplicates,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::JsonFilteringJsonPath,
    ConnectorCapability::JsonFilteringAlphanumeric,
    ConnectorCapability::CreateManyWriteableAutoIncId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::AnyId,
    ConnectorCapability::SqlQueryRaw,
    // ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::AdvancedJsonNullability,
    ConnectorCapability::IndexColumnLengthPrefixing,
    // ConnectorCapability::FullTextIndex,
    // ConnectorCapability::FullTextSearchWithIndex,
    // ConnectorCapability::MultipleFullTextAttributesPerModel,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::OrderByNullsFirstLast,
    ConnectorCapability::SupportsTxIsolationReadUncommitted,
    ConnectorCapability::SupportsTxIsolationReadCommitted,
    ConnectorCapability::SupportsTxIsolationRepeatableRead,
    // ConnectorCapability::SupportsTxIsolationSerializable,
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TiDBConnectorTag {
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for TiDBConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "mysql"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(
        &self,
        database: &str,
        _: bool,
        _is_multi_schema: bool,
        _: Option<&'static str>,
    ) -> String {
        format!("mysql://root@localhost:4000/{}", database)
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        ("tidb".to_owned(), None)
    }

    fn is_versioned(&self) -> bool {
        false
    }

    fn relation_mode(&self) -> &'static str {
        "prisma"
    }

}

impl TiDBConnectorTag {
    pub fn new() -> Self {
        Self {
            capabilities: tidb_capabilities(),
        }
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self::new()]
    }
}

fn tidb_capabilities() -> Vec<ConnectorCapability> {
    (CAPABILITIES).to_vec()
}
