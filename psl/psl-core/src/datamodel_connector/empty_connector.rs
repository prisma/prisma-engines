pub(crate) use crate::datamodel_connector::*;
use diagnostics::{DatamodelError, Span};
use enumflags2::BitFlags;
use parser_database::ScalarFieldType;

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

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        BitFlags::all()
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        enumflags2::make_bitflags!(ConnectorCapability::{
            AutoIncrement |
            CompoundIds |
            Enums |
            Json |
            ImplicitManyToManyRelation
        })
    }

    fn max_identifier_length(&self) -> usize {
        usize::MAX
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        &[]
    }

    fn scalar_type_for_native_type(
        &self,
        _native_type: &NativeTypeInstance,
        _extension_types: &dyn ExtensionTypes,
    ) -> Option<ScalarFieldType> {
        Some(ScalarFieldType::BuiltInScalar(ScalarType::String))
    }

    fn default_native_type_for_scalar_type(
        &self,
        _scalar_type: &ScalarFieldType,
        _schema: &ValidatedSchema,
    ) -> Option<NativeTypeInstance> {
        None
    }

    fn native_type_to_parts<'t>(&self, _native_type: &'t NativeTypeInstance) -> (&'t str, Cow<'t, [String]>) {
        unreachable!("EmptyDatamodelConnector::native_type_to_string()")
    }

    fn parse_native_type(
        &self,
        name: &str,
        _: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        diagnostics.push_error(DatamodelError::new_native_type_parser_error(name, span));
        None
    }

    fn validate_url(&self, _url: &str) -> Result<(), String> {
        Ok(())
    }

    fn flavour(&self) -> Flavour {
        unreachable!()
    }

    fn is_sql(&self) -> bool {
        false
    }

    fn is_mongo(&self) -> bool {
        false
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
