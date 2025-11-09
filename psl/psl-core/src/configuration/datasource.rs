use schema_ast::ast::WithSpan;

use crate::{
    datamodel_connector::{Connector, ConnectorCapabilities, RelationMode},
    diagnostics::Span,
};
use std::{any::Any, sync::Arc};

/// a `datasource` from the prisma schema.
#[derive(Clone)]
pub struct Datasource {
    pub name: String,
    /// Span of the whole datasource block (including `datasource` keyword and braces)
    pub span: Span,
    /// The provider string
    pub provider: String,
    /// Span of the provider attribute. Used by the language server.
    pub provider_span: Span,
    /// The provider that was selected as active from all specified providers
    pub active_provider: &'static str,
    pub documentation: Option<String>,
    /// the connector of the active provider
    pub active_connector: &'static dyn Connector,
    /// In which layer referential actions are handled.
    pub relation_mode: Option<RelationMode>,
    /// _Sorted_ vec of schemas defined in the schemas property.
    pub namespaces: Vec<(String, Span)>,
    pub schemas_span: Option<Span>,
    pub connector_data: DatasourceConnectorData,
}

#[derive(Clone, Default)]
pub struct DatasourceConnectorData {
    data: Option<Arc<dyn Any + Send + Sync + 'static>>,
}

impl DatasourceConnectorData {
    pub fn new(data: Arc<dyn Any + Send + Sync + 'static>) -> Self {
        Self { data: Some(data) }
    }

    #[track_caller]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.as_ref().map(|data| data.downcast_ref().unwrap())
    }
}

impl std::fmt::Debug for Datasource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Datasource")
            .field("name", &self.name)
            .field("provider", &self.provider)
            .field("active_provider", &self.active_provider)
            .field("url", &"<url>")
            .field("documentation", &self.documentation)
            .field("active_connector", &&"...")
            .field("shadow_database_url", &"<shadow_database_url>")
            .field("relation_mode", &self.relation_mode)
            .field("namespaces", &self.namespaces)
            .finish()
    }
}

impl Datasource {
    /// Extract connector-specific constructs. The type parameter must be the right one.
    #[track_caller]
    pub fn downcast_connector_data<T: 'static>(&self) -> Option<&T> {
        self.connector_data.downcast_ref()
    }

    pub(crate) fn has_schema(&self, name: &str) -> bool {
        self.namespaces.binary_search_by_key(&name, |(s, _)| s).is_ok()
    }

    pub fn capabilities(&self) -> ConnectorCapabilities {
        self.active_connector.capabilities()
    }

    /// The applicable relation mode for this datasource.
    #[allow(clippy::or_fun_call)] // not applicable in this case
    pub fn relation_mode(&self) -> RelationMode {
        self.relation_mode
            .unwrap_or(self.active_connector.default_relation_mode())
    }

    // Validation for property existence
    pub fn provider_defined(&self) -> bool {
        !self.provider.is_empty()
    }

    pub fn relation_mode_defined(&self) -> bool {
        self.relation_mode.is_some()
    }

    pub fn schemas_defined(&self) -> bool {
        self.schemas_span.is_some()
    }
}

impl WithSpan for Datasource {
    fn span(&self) -> Span {
        self.span
    }
}
