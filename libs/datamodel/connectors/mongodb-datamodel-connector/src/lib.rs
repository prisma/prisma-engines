mod mongodb_types;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability, ReferentialIntegrity,
};
use dml::{
    default_value::DefaultKind, field::FieldType, native_type_instance::NativeTypeInstance,
    relation_info::ReferentialAction, scalars::ScalarType, traits::WithDatabaseName,
};
use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::MongoDbType;
use std::result::Result as StdResult;

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::RelationsOverNonUniqueCriteria,
    ConnectorCapability::Json,
    ConnectorCapability::Enums,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::CreateMany,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::CompositeTypes,
];

type Result<T> = std::result::Result<T, ConnectorError>;

pub struct MongoDbDatamodelConnector;

impl Connector for MongoDbDatamodelConnector {
    fn name(&self) -> &str {
        "MongoDB"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    fn constraint_name_length(&self) -> usize {
        127
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        referential_integrity.allowed_referential_actions(BitFlags::empty())
    }

    fn validate_field_default(
        &self,
        field_name: &str,
        _scalar_type: &ScalarType,
        native_type: Option<&NativeTypeInstance>,
        default: Option<&dml::default_value::DefaultValue>,
        errors: &mut Vec<ConnectorError>,
    ) {
        if native_type.is_some() {
            return;
        }

        if !matches!(default.map(|d| d.kind()), Some(dml::default_value::DefaultKind::Expression(expr)) if expr.is_dbgenerated())
        {
            return;
        }

        errors.push(ConnectorError::from_kind(ErrorKind::FieldValidationError {
            field: field_name.to_owned(),
            message: "MongoDB `@default(dbgenerated())` fields must have a native type annotation.".to_owned(),
        }))
    }

    fn validate_model(&self, model: &dml::model::Model, errors: &mut Vec<ConnectorError>) {
        if let Some(pk) = &model.primary_key {
            // no compound ids
            if pk.fields.len() > 1 {
                errors.push(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                    message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
                }));
            }

            // singular id
            let field_name = pk.fields.first().unwrap();
            let field = model.find_scalar_field(field_name.name.as_str()).unwrap();

            // The _id name check is superfluous because it's not a valid schema field at the moment.
            if field.name != "_id" {
                match field.database_name() {
                    Some("_id") => (),
                    Some(mapped_name) => errors.push(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name.to_owned(),
                        message: format!(
                            "MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{}\").",
                            mapped_name
                        ),
                    })),
                    None => errors.push(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name.to_owned(),
                        message: "MongoDB model IDs must have a @map(\"_id\") annotations.".to_owned(),
                    })),
                };
            }

            if !matches!(field.field_type, FieldType::Scalar(_, _, Some(_)))
                && matches!(field.default_value.as_ref().map(|v| v.kind()), Some(DefaultKind::Expression(expr)) if expr.is_dbgenerated())
            {
                errors.push(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                    field: field.name.to_owned(),
                    message: format!(
                        "MongoDB `@default(dbgenerated())` IDs must have an `ObjectID` native type annotation. `{}` is an ID field, so you probably want `ObjectId` as your native type.",
                        field.name
                    ),
                }));
            }
        } else {
            errors.push(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
            }))
        }
    }

    fn available_native_type_constructors(&self) -> &'static [dml::native_type_constructor::NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
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

    fn default_referential_integrity(&self) -> ReferentialIntegrity {
        ReferentialIntegrity::Prisma
    }

    fn allowed_referential_integrity_settings(&self) -> enumflags2::BitFlags<ReferentialIntegrity> {
        ReferentialIntegrity::Prisma.into()
    }
}
