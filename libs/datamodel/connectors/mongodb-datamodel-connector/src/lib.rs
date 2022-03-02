mod mongodb_types;

use datamodel_connector::{
    parser_database::{ast::Expression, walkers::*},
    Connector, ConnectorCapability, ConstraintScope, DatamodelError, Diagnostics, NativeTypeConstructor,
    NativeTypeInstance, ReferentialAction, ReferentialIntegrity, ScalarType, Span,
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

type Result<T> = std::result::Result<T, DatamodelError>;

pub struct MongoDbDatamodelConnector;

impl MongoDbDatamodelConnector {
    fn validate_auto(field: ScalarFieldWalker<'_>, errors: &mut datamodel_connector::Diagnostics) {
        if !field.default_value().map(|val| val.is_auto()).unwrap_or(false) {
            return;
        }

        let mut bail = || {
            let err = DatamodelError::new_field_validation_error(
                "MongoDB `@default(auto())` fields must have `ObjectId` native type and use the `@id` attribute.",
                field.model().name(),
                field.name(),
                field.ast_field().span,
            );
            errors.push_error(err);
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

        let err = DatamodelError::new_field_validation_error(
            "The `dbgenerated()` function is not allowed with MongoDB. Please use `auto()` instead.",
            field.model().name(),
            field.name(),
            field.ast_field().span,
        );
        errors.push_error(err);
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

        errors.push_error(DatamodelError::new_field_validation_error(
            &format!(
                "Native type `{ds_name}.{}` is deprecated. Please use `{ds_name}.{arg}` instead.",
                type_names::ARRAY
            ),
            field.model().name(),
            field.name(),
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

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        &[ConstraintScope::ModelKeyIndex]
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

        if let Some(pk) = model.primary_key() {
            // no compound ids
            if pk.fields().len() > 1 {
                errors.push_error(DatamodelError::new_invalid_model_error(
                    "MongoDB models require exactly one identity field annotated with @id",
                    model.ast_model().span,
                ));
            }

            // singular id
            let field = pk.fields().next().unwrap();

            // The _id name check is superfluous because it's not a valid schema field at the moment.
            if field.name() != "_id" {
                match field.mapped_name() {
                    Some("_id") => (),
                    Some(mapped_name) => errors.push_error(DatamodelError::new_field_validation_error(
                        &format!(
                            "MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{}\").",
                            mapped_name
                        ),
                        field.model().name(),
                        field.name(),
                        field.ast_field().span,
                    )),
                    None => errors.push_error(DatamodelError::new_field_validation_error(
                        "MongoDB model IDs must have a @map(\"_id\") annotations.",
                        field.model().name(),
                        field.name(),
                        field.ast_field().span,
                    )),
                };
            }

            if field.raw_native_type().is_none()
                && matches!(field.default_value().map(|v| v.value()), Some(Expression::Function(fn_name,_,_)) if fn_name == "dbgenerated")
            {
                errors.push_error(DatamodelError::new_field_validation_error(
                    &format!(
                        "MongoDB `@default(dbgenerated())` IDs must have an `ObjectID` native type annotation. `{}` is an ID field, so you probably want `ObjectId` as your native type.",
                        field.name()
                        ),
                        field.model().name(),
                        field.name(),
                        field.ast_field().span,

                ));
            }
        } else {
            errors.push_error(DatamodelError::new_invalid_model_error(
                "MongoDB models require exactly one identity field annotated with @id",
                model.ast_model().span,
            ));
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

    fn parse_native_type(&self, name: &str, args: Vec<String>, span: Span) -> Result<NativeTypeInstance> {
        let mongo_type = mongo_type_from_input(name, &args, span)?;

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
