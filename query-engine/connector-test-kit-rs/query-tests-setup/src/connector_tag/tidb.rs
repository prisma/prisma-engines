use super::*;
use crate::SqlDatamodelRenderer;
use psl::datamodel_connector::ConnectorCapability;

const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    Enums |
    EnumArrayPush |
    Json |
    AutoIncrementAllowedOnNonId |
    RelationFieldsInArbitraryOrder |
    CreateMany |
    WritableAutoincField |
    CreateSkipDuplicates |
    UpdateableId |
    JsonFiltering |
    JsonFilteringJsonPath |
    JsonFilteringAlphanumeric |
    CreateManyWriteableAutoIncId |
    AutoIncrement |
    CompoundIds |
    AnyId |
    SqlQueryRaw |
    NamedForeignKeys |
    AdvancedJsonNullability |
    IndexColumnLengthPrefixing |
    MultiSchema |
    ImplicitManyToManyRelation |
    DecimalType |
    OrderByNullsFirstLast |
    SupportsTxIsolationReadUncommitted |
    SupportsTxIsolationReadCommitted |
    SupportsTxIsolationRepeatableRead
});

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

    fn connection_string(&self, database: &str, _: bool, _is_multi_schema: bool, _: Option<&'static str>) -> String {
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
}

impl TiDBConnectorTag {
    pub fn new() -> Self {
        Self
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self::new()]
    }
}
