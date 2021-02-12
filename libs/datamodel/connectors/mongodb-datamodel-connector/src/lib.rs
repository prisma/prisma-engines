use datamodel_connector::{Connector, ConnectorCapability};

pub struct MongoDbDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
}

impl MongoDbDatamodelConnector {
    pub fn new() -> Self {
        let capabilities = vec![
            ConnectorCapability::RelationsOverNonUniqueCriteria,
            ConnectorCapability::Json,
            ConnectorCapability::MultipleIndexesWithSameName,
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::CreateMany,
            ConnectorCapability::CreateSkipDuplicates,
            ConnectorCapability::ScalarLists,
        ];

        Self { capabilities }
    }
}

impl Connector for MongoDbDatamodelConnector {
    fn name(&self) -> String {
        "MongoDB".to_owned()
    }

    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn validate_field(
        &self,
        _field: &dml::field::Field,
    ) -> Result<(), datamodel_connector::connector_error::ConnectorError> {
        Ok(())
    }

    fn validate_model(
        &self,
        _model: &dml::model::Model,
    ) -> Result<(), datamodel_connector::connector_error::ConnectorError> {
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &[dml::native_type_constructor::NativeTypeConstructor] {
        &[]
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &dml::scalars::ScalarType) -> serde_json::Value {
        todo!()
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: serde_json::Value,
        _scalar_type: &dml::scalars::ScalarType,
    ) -> bool {
        todo!()
    }

    fn parse_native_type(
        &self,
        _name: &str,
        _args: Vec<String>,
    ) -> Result<dml::native_type_instance::NativeTypeInstance, datamodel_connector::connector_error::ConnectorError>
    {
        todo!()
    }

    fn introspect_native_type(
        &self,
        _native_type: serde_json::Value,
    ) -> Result<dml::native_type_instance::NativeTypeInstance, datamodel_connector::connector_error::ConnectorError>
    {
        todo!()
    }
}
