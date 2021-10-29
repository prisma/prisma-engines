use crate::{connector_error::ConnectorError, Connector, ConnectorCapability};
use dml::relation_info::ReferentialAction;
use enumflags2::BitFlags;

/// A [Connector](/trait.Connector.html) implementor meant to
/// be used as a default when no datasource is defined.
pub struct EmptyDatamodelConnector;

impl Connector for EmptyDatamodelConnector {
    fn name(&self) -> &str {
        std::any::type_name::<EmptyDatamodelConnector>()
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        BitFlags::all()
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &[
            ConnectorCapability::CompoundIds,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
        ]
    }

    fn constraint_name_length(&self) -> usize {
        usize::MAX
    }

    fn available_native_type_constructors(&self) -> &[dml::native_type_constructor::NativeTypeConstructor] {
        &[]
    }

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> dml::scalars::ScalarType {
        dml::scalars::ScalarType::String
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &dml::scalars::ScalarType) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: serde_json::Value,
        _scalar_type: &dml::scalars::ScalarType,
    ) -> bool {
        false
    }

    fn parse_native_type(
        &self,
        name: &str,
        _args: Vec<String>,
    ) -> Result<dml::native_type_instance::NativeTypeInstance, ConnectorError> {
        Err(ConnectorError::new_native_type_parser_error(name))
    }

    fn introspect_native_type(
        &self,
        _native_type: serde_json::Value,
    ) -> Result<dml::native_type_instance::NativeTypeInstance, ConnectorError> {
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
