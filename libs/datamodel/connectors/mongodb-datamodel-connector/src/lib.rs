mod mongodb_types;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::{
    native_type_constructor::NativeTypeConstructor, native_type_instance::NativeTypeInstance,
    relation_info::ReferentialAction, traits::WithDatabaseName,
};
use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::MongoDbType;
use std::result::Result as StdResult;

type Result<T> = std::result::Result<T, ConnectorError>;

pub struct MongoDbDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    native_types: Vec<NativeTypeConstructor>,
}

impl MongoDbDatamodelConnector {
    pub fn new() -> Self {
        let capabilities = vec![
            ConnectorCapability::RelationsOverNonUniqueCriteria,
            ConnectorCapability::Json,
            ConnectorCapability::Enums,
            ConnectorCapability::MultipleIndexesWithSameName,
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::CreateMany,
            ConnectorCapability::CreateSkipDuplicates,
            ConnectorCapability::ScalarLists,
            ConnectorCapability::InsensitiveFilters,
        ];

        let native_types = mongodb_types::available_types();

        Self {
            capabilities,
            native_types,
        }
    }
}

impl Default for MongoDbDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Connector for MongoDbDatamodelConnector {
    fn name(&self) -> String {
        "MongoDB".to_owned()
    }

    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        BitFlags::empty()
    }

    fn validate_field(&self, field: &dml::field::Field) -> Result<()> {
        // WIP, I don't really know what I'm doing with the dml.

        // The _id name check is superfluous because it's not a valid schema field at the moment.
        if field.is_id() && field.name() != "_id" {
            match field.database_name() {
                Some(mapped_name) if mapped_name != "_id" => {
                    Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name().to_owned(),
                        message: format!(
                            "MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{}\").",
                            mapped_name
                        ),
                    }))
                }
                Some(_) => Ok(()),
                None => Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                    field: field.name().to_owned(),
                    message: "MongoDB model IDs must have a @map(\"_id\") annotations.".to_owned(),
                })),
            }?;
        }

        Ok(())
    }

    fn validate_model(&self, _model: &dml::model::Model) -> Result<()> {
        // Todo
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &[dml::native_type_constructor::NativeTypeConstructor] {
        &self.native_types
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &dml::scalars::ScalarType) -> serde_json::Value {
        let native_type = default_for(scalar_type);
        serde_json::to_value(native_type).expect("MongoDB native type to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: serde_json::Value,
        scalar_type: &dml::scalars::ScalarType,
    ) -> bool {
        let default_native_type = default_for(scalar_type);
        let native_type: MongoDbType =
            serde_json::from_value(native_type).expect("MongoDB native type from JSON failed");

        &native_type == default_native_type
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: Vec<String>,
    ) -> Result<dml::native_type_instance::NativeTypeInstance> {
        let mongo_type = mongo_type_from_input(name, &args)?;

        Ok(NativeTypeInstance::new(name, args, &mongo_type))
    }

    fn introspect_native_type(
        &self,
        _native_type: serde_json::Value,
    ) -> Result<dml::native_type_instance::NativeTypeInstance> {
        // Out of scope for MVP
        todo!()
    }

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> dml::scalars::ScalarType {
        // Out of scope for MVP
        todo!()
    }

    fn validate_url(&self, url: &str) -> StdResult<(), String> {
        if !url.starts_with("mongo") {
            return Err("must start with the protocol `mongo`.".into());
        }

        Ok(())
    }
}
