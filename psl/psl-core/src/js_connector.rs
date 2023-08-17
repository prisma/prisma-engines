use crate::datamodel_connector::*;
use enumflags2::BitFlags;

/// JsConnector represents a type of connector that is implemented partially
/// in javascript and used from rust through the js-connectors crate
///
/// Rather than a unit struct per individual connector, like we have for the rest
/// of the builtin connectors, we have a single struct which state represents the
/// features that vary in this connector with respect to a cannonical connector
/// for the flavour of SQL the particular JsConnector speaks.
///
/// For example, the _planetscale serverless_ connector is compatible with MySQL,
/// so it reuses the builtin MySQL connector (the cannonical for the MySQL flavour)
/// for most of its features.
#[derive(Copy, Clone)]
pub struct JsConnector {
    pub flavour: Flavour,
    pub canonical_connector: &'static dyn Connector,

    pub provider_name: &'static str,
    pub name: &'static str,
    pub allowed_protocols: Option<&'static [&'static str]>,
}

impl JsConnector {
    /// Returns true if the given name is a valid provider name for a JsConnector.
    /// We use the convention that if a provider starts with ´@prisma/´ (ex. ´@prisma/planetscale´)
    /// then its a provider for a JS connector.
    pub fn is_provider(name: &str) -> bool {
        name.starts_with("@prisma/")
    }
}

impl Connector for JsConnector {
    fn as_js_connector(&self) -> Option<JsConnector> {
        Some(*self)
    }

    fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    fn name(&self) -> &str {
        self.name
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        self.canonical_connector.capabilities()
    }

    fn max_identifier_length(&self) -> usize {
        self.canonical_connector.max_identifier_length()
    }

    fn referential_actions(&self) -> enumflags2::BitFlags<parser_database::ReferentialAction> {
        self.canonical_connector.referential_actions()
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        self.canonical_connector.available_native_type_constructors()
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> parser_database::ScalarType {
        self.canonical_connector.scalar_type_for_native_type(native_type)
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &parser_database::ScalarType) -> NativeTypeInstance {
        self.canonical_connector
            .default_native_type_for_scalar_type(scalar_type)
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &parser_database::ScalarType,
    ) -> bool {
        self.canonical_connector
            .native_type_is_default_for_scalar_type(native_type, scalar_type)
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        self.canonical_connector.native_type_to_parts(native_type)
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: diagnostics::Span,
        diagnostics: &mut diagnostics::Diagnostics,
    ) -> Option<NativeTypeInstance> {
        self.canonical_connector
            .parse_native_type(name, args, span, diagnostics)
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if let Some(allowed_protocols) = self.allowed_protocols {
            let scheme = url.split(':').next().unwrap_or("");
            if allowed_protocols.contains(&scheme) {
                Ok(())
            } else {
                Err(format!(
                    "The URL scheme `{}` is not valid for the {} connector. The following schemes are allowed: {}",
                    scheme,
                    self.name,
                    allowed_protocols.join(", ")
                ))
            }
        } else {
            self.canonical_connector.validate_url(url)
        }
    }

    fn default_relation_mode(&self) -> RelationMode {
        self.canonical_connector.default_relation_mode()
    }

    fn allowed_relation_mode_settings(&self) -> BitFlags<RelationMode> {
        self.canonical_connector.allowed_relation_mode_settings()
    }

    fn flavour(&self) -> Flavour {
        self.flavour
    }
}
