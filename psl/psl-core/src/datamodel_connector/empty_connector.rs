use crate::datamodel_connector::*;
use diagnostics::{DatamodelError, Span};
use enumflags2::BitFlags;

/// A [Connector](/trait.Connector.html) implementor meant to
/// be used as a default when no datasource is defined.
pub struct EmptyDatamodelConnector;

impl Connector for EmptyDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "empty"
    }

    fn name(&self) -> &str {
        std::any::type_name::<EmptyDatamodelConnector>()
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        BitFlags::all()
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        &[
            ConnectorCapability::AutoIncrement,
            ConnectorCapability::CompoundIds,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
            ConnectorCapability::ImplicitManyToManyRelation,
        ]
    }

    fn max_identifier_length(&self) -> usize {
        usize::MAX
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        &[]
    }

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> ScalarType {
        ScalarType::String
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &ScalarType) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: serde_json::Value,
        _scalar_type: &ScalarType,
    ) -> bool {
        false
    }

    fn parse_native_type(&self, name: &str, _: Vec<String>, span: Span) -> Result<NativeTypeInstance, DatamodelError> {
        Err(DatamodelError::new_native_type_parser_error(name, span))
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> NativeTypeInstance {
        unreachable!("introspect_native_type on EmptyDatamodelConnector")
    }

    fn validate_url(&self, _url: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_datamodel_connector_as_dyn_connector() {
        let _connector: &dyn Connector = &EmptyDatamodelConnector;
    }
}
