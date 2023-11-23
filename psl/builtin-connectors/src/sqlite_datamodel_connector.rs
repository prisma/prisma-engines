mod native_types;
pub use native_types::SQLiteType;

use enumflags2::BitFlags;
use psl_core::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{ReferentialAction, ScalarType},
};
use std::borrow::Cow;

use crate::geometry::{GeometryParams, GeometryType};

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalKeyIndex];
const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    AnyId |
    AutoIncrement |
    CompoundIds |
    EwktGeometry |
    GeoJsonGeometry |
    GeometryRawRead |
    GeometryFiltering |
    GeometryExtraDims |
    SqlQueryRaw |
    RelationFieldsInArbitraryOrder |
    UpdateableId |
    ImplicitManyToManyRelation |
    DecimalType |
    BackwardCompatibleQueryRaw |
    OrderByNullsFirstLast |
    SupportsTxIsolationSerializable |
    NativeUpsert |
    FilteredInlineChildNestedToOneDisconnect |
    RowIn
    // InsertReturning - While SQLite does support RETURNING, it does not return column information on the way back from the database.
    // This column type information is necessary in order to preserve consistency for some data types such as int, where values could overflow.
    // Since we care to stay consistent with reads, it is not enabled.
});

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, SQLiteType)] = &[
    (
        ScalarType::Geometry,
        SQLiteType::Geometry(Some(GeometryParams {
            ty: GeometryType::Geometry,
            srid: 0,
        })),
    ),
    (
        ScalarType::GeoJson,
        SQLiteType::Geometry(Some(GeometryParams {
            ty: GeometryType::Geometry,
            srid: 4326,
        })),
    ),
];

pub struct SqliteDatamodelConnector;

impl Connector for SqliteDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "sqlite"
    }

    fn name(&self) -> &str {
        "sqlite"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        10000
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        SetNull | SetDefault | Cascade | Restrict | NoAction
    }

    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | Cascade
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        let native_type: &SQLiteType = native_type.downcast_ref();
        match native_type {
            SQLiteType::Geometry(_) => ScalarType::Geometry,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> NativeTypeInstance {
        SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .map(|nt| NativeTypeInstance::new::<SQLiteType>(*nt))
            .unwrap_or(NativeTypeInstance::new(()))
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let native_type: &SQLiteType = native_type.downcast_ref();

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &SQLiteType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            SQLiteType::Geometry(Some(g)) if *scalar_type == ScalarType::GeoJson && g.srid != 4326 => {
                errors.push_error(error.new_argument_m_out_of_range_error("GeoJson SRID must be 4326.", span))
            }
            SQLiteType::Geometry(Some(g)) if g.srid < -1 => errors
                .push_error(error.new_argument_m_out_of_range_error("SRID must be superior or equal to -1.", span)),
            SQLiteType::Geometry(Some(g)) if g.ty.is_extra() => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    &format!("{} isn't supported for the current connector.", g.ty),
                    span,
                ))
            }
            _ => (),
        }
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        native_type.downcast_ref::<SQLiteType>().to_parts()
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        native_types::CONSTRUCTORS
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        SQLiteType::from_parts(name, args, span, diagnostics).map(NativeTypeInstance::new::<SQLiteType>)
    }

    fn set_config_dir<'a>(&self, config_dir: &std::path::Path, url: &'a str) -> Cow<'a, str> {
        let set_root = |path: &str| {
            let path = std::path::Path::new(path);

            if path.is_relative() {
                Some(config_dir.join(path).to_str().map(ToString::to_string).unwrap())
            } else {
                None
            }
        };

        if let Some(path) = set_root(url.trim_start_matches("file:")) {
            return Cow::Owned(format!("file:{path}"));
        };

        Cow::Borrowed(url)
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("file") {
            return Err("must start with the protocol `file:`.".to_string());
        }

        Ok(())
    }

    fn flavour(&self) -> Flavour {
        Flavour::Sqlite
    }
}
