mod mongodb_types;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    parser_database::{ast::Expression, walkers::*},
    Connector, ConnectorCapability, DatamodelError, Diagnostics, NativeTypeConstructor, NativeTypeInstance,
    ReferentialAction, ReferentialIntegrity, ScalarType,
};
use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::{MongoDbType, NativeType};
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
    ConnectorCapability::MongoDbQueryRaw,
    ConnectorCapability::DefaultValueAuto,
    ConnectorCapability::TwoWayEmbeddedManyToManyRelation,
];

type Result<T> = std::result::Result<T, ConnectorError>;

pub struct MongoDbDatamodelConnector;

impl MongoDbDatamodelConnector {
    fn validate_auto(field: ScalarFieldWalker<'_>, errors: &mut datamodel_connector::Diagnostics) {
        if !field.default_value().map(|val| val.is_auto()).unwrap_or(false) {
            return;
        }

        let mut bail = || {
            let err = ConnectorError::from_kind(ErrorKind::FieldValidationError {
                field: field.name().to_owned(),
                message:
                    "MongoDB `@default(auto())` fields must have `ObjectId` native type and use the `@id` attribute."
                        .to_owned(),
            });
            errors.push_error(DatamodelError::new_connector_error(err, field.ast_field().span))
        };

        let model = field.model();
        let is_id = model.field_is_single_pk(field.field_id());

        match field.raw_native_type() {
            None => bail(),
            Some((_, name, _, _)) if name != "ObjectId" => bail(),
            _ if !is_id => bail(),
            _ => (),
        }
    }

    fn validate_dbgenerated(field: ScalarFieldWalker<'_>, errors: &mut datamodel_connector::Diagnostics) {
        if !field.default_value().map(|val| val.is_dbgenerated()).unwrap_or(false) {
            return;
        }

        let err = ConnectorError::from_kind(ErrorKind::FieldValidationError {
            field: field.name().to_owned(),
            message: "The `dbgenerated()` function is not allowed with MongoDB. Please use `auto()` instead."
                .to_owned(),
        });
        errors.push_error(DatamodelError::new_connector_error(err, field.ast_field().span));
    }

    fn validate_array_native_type(field: ScalarFieldWalker<'_>, errors: &mut Diagnostics) {
        let (ds_name, type_name, args, span) = match field.raw_native_type() {
            Some(nt) => nt,
            None => return,
        };

        if type_name != type_names::ARRAY {
            return;
        }

        // `db.Array` expects exactly 1 argument, which is validated before this code path.
        let arg = args.get(0).unwrap();

        errors.push_error(DatamodelError::new_connector_error(
            ConnectorError::from_kind(ErrorKind::FieldValidationError {
                field: field.name().to_owned(),
                message: format!(
                    "Native type `{ds_name}.{}` is deprecated. Please use `{ds_name}.{arg}` instead.",
                    type_names::ARRAY
                ),
            }),
            span,
        ));
    }
}

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

    fn validate_model(&self, model: ModelWalker<'_>, errors: &mut Diagnostics) {
        for field in model.scalar_fields() {
            Self::validate_auto(field, errors);
            Self::validate_dbgenerated(field, errors);
            Self::validate_array_native_type(field, errors);
        }

        let mut push_error = |err: ConnectorError| {
            errors.push_error(DatamodelError::new_connector_error(err, model.ast_model().span));
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
                && matches!(field.default_value().map(|v| v.value()), Some(Expression::Function(fn_name,_,_)) if fn_name == "dbgenerated")
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

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance> {
        let mongo_type = mongo_type_from_input(name, &args)?;

        Ok(NativeTypeInstance::new(name, args, mongo_type.to_json()))
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> Result<NativeTypeInstance> {
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
