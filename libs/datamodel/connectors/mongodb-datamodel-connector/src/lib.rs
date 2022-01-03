mod mongodb_types;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    parser_database::walkers::*,
    walker_ext_traits::*,
    Connector, ConnectorCapability, NativeTypeConstructor, ReferentialAction, ReferentialIntegrity, ScalarType,
};
use dml::{default_value::DefaultKind, native_type_instance::NativeTypeInstance};
use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::MongoDbType;
use std::result::Result as StdResult;

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::RelationsOverNonUniqueCriteria,
    ConnectorCapability::Json,
    ConnectorCapability::Enums,
    ConnectorCapability::EnumArrayPush,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::CreateMany,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::JsonLists,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::CompositeTypes,
    ConnectorCapability::FullTextIndex,
    ConnectorCapability::SortOrderInFullTextIndex,
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

    fn max_identifier_length(&self) -> usize {
        127
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        referential_integrity.allowed_referential_actions(BitFlags::empty())
    }

    fn validate_model(&self, model: ModelWalker<'_, '_>, errors: &mut datamodel_connector::Diagnostics) {
        for field in model.scalar_fields() {
            if field.raw_native_type().is_none()
                && field
                    .default_value()
                    .map(|val| val.is_dbgenerated())
                    .unwrap_or_default()
            {
                errors.push_error(datamodel_connector::DatamodelError::ConnectorError {
                    message: ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name().to_owned(),
                        message: "MongoDB `@default(dbgenerated())` fields must have a native type annotation."
                            .to_owned(),
                    })
                    .to_string(),
                    span: field.ast_field().span,
                })
            }
        }

        let mut push_error = |err: ConnectorError| {
            errors.push_error(datamodel_connector::DatamodelError::ConnectorError {
                message: err.to_string(),
                span: model.ast_model().span,
            });
        };

        if let Some(pk) = model.primary_key() {
            // no compound ids
            if pk.fields().len() > 1 {
                push_error(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                    message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
                }));
            }

            // singular id
            let field = pk.fields().next().unwrap();

            // The _id name check is superfluous because it's not a valid schema field at the moment.
            if field.name() != "_id" {
                match field.mapped_name() {
                    Some("_id") => (),
                    Some(mapped_name) => push_error(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name().to_owned(),
                        message: format!(
                            "MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{}\").",
                            mapped_name
                        ),
                    })),
                    None => push_error(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                        field: field.name().to_owned(),
                        message: "MongoDB model IDs must have a @map(\"_id\") annotations.".to_owned(),
                    })),
                };
            }

            if field.raw_native_type().is_none()
                && matches!(field.default_value().map(|v| v.dml_default_kind()), Some(DefaultKind::Expression(expr)) if expr.is_dbgenerated())
            {
                push_error(ConnectorError::from_kind(ErrorKind::FieldValidationError {
                    field: field.name().to_owned(),
                    message: format!(
                        "MongoDB `@default(dbgenerated())` IDs must have an `ObjectID` native type annotation. `{}` is an ID field, so you probably want `ObjectId` as your native type.",
                        field.name()
                    ),
                }));
            }
        } else {
            push_error(ConnectorError::from_kind(ErrorKind::InvalidModelError {
                message: "MongoDB models require exactly one identity field annotated with @id".to_owned(),
            }))
        }
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = default_for(scalar_type);
        serde_json::to_value(native_type).expect("MongoDB native type to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
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

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> ScalarType {
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
