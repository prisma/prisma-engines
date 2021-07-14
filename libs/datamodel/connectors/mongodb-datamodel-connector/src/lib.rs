mod mongodb_types;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::{
    default_value::DefaultValue, field::FieldType, native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance, relation_info::ReferentialAction, traits::WithDatabaseName,
};
use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::MongoDbType;
use std::result::Result as StdResult;

type Result<T> = std::result::Result<T, ConnectorError>;

pub struct MongoDbDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    native_types: Vec<NativeTypeConstructor>,
    referential_actions: BitFlags<ReferentialAction>,
}

impl MongoDbDatamodelConnector {
    pub fn new() -> Self {
        use ReferentialAction::*;

        let capabilities = vec![
            ConnectorCapability::RelationsOverNonUniqueCriteria,
            ConnectorCapability::Json,
            ConnectorCapability::Enums,
            ConnectorCapability::MultipleIndexesWithSameName,
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::CreateMany,
            ConnectorCapability::ScalarLists,
            ConnectorCapability::InsensitiveFilters,
        ];

        let native_types = mongodb_types::available_types();
        let referential_actions = Restrict | SetNull | NoAction | Cascade;

        Self {
            capabilities,
            native_types,
            referential_actions,
        }
    }
}

impl Default for MongoDbDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Connector for MongoDbDatamodelConnector {
    fn name(&self) -> &str {
        "MongoDB"
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn constraint_name_length(&self) -> usize {
        127
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        self.referential_actions
    }

    fn emulates_referential_actions(&self) -> bool {
        true
    }

    fn validate_field(&self, field: &dml::field::Field) -> Result<()> {
        // If the field is _not_ a native-type-annotated field and it has a `dbgenerated` default, we error.
        if !matches!(field.field_type(), FieldType::Scalar(_, _, Some(_)))
            && matches!(field.default_value(), Some(DefaultValue::Expression(expr)) if expr.is_dbgenerated())
        {
            return Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                field: field.name().to_owned(),
                message: "MongoDB `@default(dbgenerated())` fields must have a native type annotation.".to_owned(),
            }));
        }

        Ok(())
    }

    fn validate_model(&self, model: &dml::model::Model) -> Result<()> {
        if let Some(pk) = &model.primary_key {
            // no compound ids
            if pk.fields.len() > 1 {
                return Err(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                    message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
                }));
            }

            // singular id
            let field_name = pk.fields.first().unwrap();
            let field = model.find_scalar_field(&field_name).unwrap();

            // The _id name check is superfluous because it's not a valid schema field at the moment.
            if field.name != "_id" {
                match field.database_name() {
                    Some("_id") => Ok(()),
                    Some(mapped_name) => Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name.to_owned(),
                        message: format!(
                            "MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{}\").",
                            mapped_name
                        ),
                    })),
                    None => Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name.to_owned(),
                        message: "MongoDB model IDs must have a @map(\"_id\") annotations.".to_owned(),
                    })),
                }?;
            }

            if !matches!(field.field_type, FieldType::Scalar(_, _, Some(_)))
                && matches!(&field.default_value, Some(DefaultValue::Expression(expr)) if expr.is_dbgenerated())
            {
                return Err(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                    field: field.name.to_owned(),
                    message: format!(
                        "MongoDB `@default(dbgenerated())` IDs must have an `ObjectID` native type annotation. `{}` is an ID field, so you probably want `ObjectId` as your native type.",
                        field.name
                    ),
                }));
            }

            Ok(())
        } else {
            Err(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
            }))
        }
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
