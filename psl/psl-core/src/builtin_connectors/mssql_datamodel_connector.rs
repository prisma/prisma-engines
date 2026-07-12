mod native_types;
mod validations;

use std::borrow::Cow;

pub use native_types::{MsSqlType, MsSqlTypeParameter};
use parser_database::{ExtensionTypes, ScalarFieldType};

use crate::{
    ValidatedSchema,
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance, RelationMode,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{self, ParserDatabase, ReferentialAction, ScalarType, ast},
};
use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};

use MsSqlType::*;
use MsSqlTypeParameter::*;

use super::completions;

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[
    ConstraintScope::GlobalPrimaryKeyForeignKeyDefault,
    ConstraintScope::ModelPrimaryKeyKeyIndex,
];

const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    AnyId |
    AutoIncrement |
    AutoIncrementAllowedOnNonId |
    AutoIncrementMultipleAllowed |
    AutoIncrementNonIndexedAllowed |
    CompoundIds |
    CreateMany |
    MultiSchema |
    NamedDefaultValues |
    NamedForeignKeys |
    NamedPrimaryKeys |
    ReferenceCycleDetection |
    UpdateableId |
    PrimaryKeySortOrderDefinition |
    ImplicitManyToManyRelation |
    DecimalType |
    ClusteringSetting |
    OrderByNullsFirstLast |
    FilteredInlineChildNestedToOneDisconnect |
    SupportsTxIsolationReadUncommitted |
    SupportsTxIsolationReadCommitted |
    SupportsTxIsolationRepeatableRead |
    SupportsTxIsolationSerializable |
    SupportsTxIsolationSnapshot |
    SupportsFiltersOnRelationsWithoutJoins |
    SupportsDefaultInInsert |
    PartialIndex
    // InsertReturning | DeleteReturning - unimplemented.
});

pub(crate) struct MsSqlDatamodelConnector;

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, MsSqlType)] = &[
    (ScalarType::Int, MsSqlType::Int),
    (ScalarType::BigInt, MsSqlType::BigInt),
    (ScalarType::Float, MsSqlType::Float(Some(53))),
    (ScalarType::Decimal, MsSqlType::Decimal(Some((32, 16)))),
    (ScalarType::Boolean, MsSqlType::Bit),
    (
        ScalarType::String,
        MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
    ),
    (ScalarType::DateTime, MsSqlType::DateTime2),
    (ScalarType::Bytes, MsSqlType::VarBinary(Some(MsSqlTypeParameter::Max))),
    (
        ScalarType::Json,
        MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
    ),
];

impl Connector for MsSqlDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "sqlserver"
    }

    fn name(&self) -> &str {
        "SQL Server"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        128
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        NoAction | Cascade | SetNull | SetDefault
    }

    fn scalar_type_for_native_type(
        &self,
        native_type: &NativeTypeInstance,
        _extension_types: &dyn ExtensionTypes,
    ) -> Option<ScalarFieldType> {
        let native_type: &MsSqlType = native_type.downcast_ref();

        let res = match native_type {
            //String
            Char(_) => ScalarType::String,
            NChar(_) => ScalarType::String,
            VarChar(_) => ScalarType::String,
            NVarChar(_) => ScalarType::String,
            Text => ScalarType::String,
            NText => ScalarType::String,
            Xml => ScalarType::String,
            UniqueIdentifier => ScalarType::String,
            //Boolean
            //Int
            TinyInt => ScalarType::Int,
            SmallInt => ScalarType::Int,
            Int => ScalarType::Int,
            //BigInt
            BigInt => ScalarType::Int,
            //Float
            Float(_) => ScalarType::Float,
            SmallMoney => ScalarType::Float,
            Money => ScalarType::Float,
            Real => ScalarType::Float,
            //Decimal
            Decimal(_) => ScalarType::Decimal,
            //DateTime
            Date => ScalarType::DateTime,
            Time => ScalarType::DateTime,
            DateTime => ScalarType::DateTime,
            DateTime2 => ScalarType::DateTime,
            SmallDateTime => ScalarType::DateTime,
            DateTimeOffset => ScalarType::DateTime,
            //Json -> does not really exist
            //Bytes
            Binary(_) => ScalarType::Bytes,
            VarBinary(_) => ScalarType::Bytes,
            Image => ScalarType::Bytes,
            Bit => ScalarType::Bytes,
        };
        Some(ScalarFieldType::BuiltInScalar(res))
    }

    fn default_native_type_for_scalar_type(
        &self,
        scalar_type: &ScalarFieldType,
        _schema: &ValidatedSchema,
    ) -> Option<NativeTypeInstance> {
        let scalar_type = scalar_type.as_builtin_scalar()?;
        let nt = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == &scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {scalar_type:?} in SCALAR_TYPE_DEFAULTS"))
            .unwrap();

        Some(NativeTypeInstance::new::<MsSqlType>(*nt))
    }

    fn validate_native_type_arguments(
        &self,
        native_type: &NativeTypeInstance,
        _scalar_type: Option<ScalarType>,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let r#type: &MsSqlType = native_type.downcast_ref();
        let error = self.native_instance_error(native_type);

        match r#type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span));
            }
            Decimal(Some((prec, _))) if *prec == 0 || *prec > 38 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 38.", span));
            }
            Decimal(Some((_, scale))) if *scale > 38 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 38.", span))
            }
            Float(Some(bits)) if *bits == 0 || *bits > 53 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Bits can range from 1 to 53.", span))
            }
            NVarChar(Some(Number(p))) if *p > 4000 => errors.push_error(error.new_argument_m_out_of_range_error(
                "Length can range from 1 to 4000. For larger sizes, use the `Max` variant.",
                span,
            )),
            VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if *p > 8000 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    r#"Length can range from 1 to 8000. For larger sizes, use the `Max` variant."#,
                    span,
                ))
            }
            NChar(Some(p)) if *p > 4000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 4000.", span))
            }
            Char(Some(p)) | Binary(Some(p)) if *p > 8000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 8000.", span))
            }
            _ => (),
        }
    }

    fn validate_model(
        &self,
        model: parser_database::walkers::ModelWalker<'_>,
        _: RelationMode,
        errors: &mut Diagnostics,
    ) {
        for index in model.indexes() {
            validations::index_uses_correct_field_types(self, index, errors);
        }

        if let Some(pk) = model.primary_key() {
            validations::primary_key_uses_correct_field_types(self, pk, errors);
        }
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
        match MsSqlType::from_parts(name, args) {
            Ok(res) => Some(NativeTypeInstance::new(res)),
            Err(err) => {
                diagnostics.push_error(err.into_datamodel_error(span));
                None
            }
        }
    }

    fn native_type_to_parts<'t>(&self, native_type: &'t NativeTypeInstance) -> (&'t str, Cow<'t, [String]>) {
        native_type.downcast_ref::<MsSqlType>().to_parts()
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("sqlserver") {
            return Err("must start with the protocol `sqlserver://`.".to_string());
        }

        Ok(())
    }

    fn datamodel_completions(
        &self,
        _db: &ParserDatabase,
        position: ast::SchemaPosition<'_>,
        completions: &mut CompletionList,
    ) {
        if let ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(
                _,
                ast::FieldPosition::Attribute("default", _, ast::AttributePosition::Attribute),
            ),
        ) = position
        {
            completions.items.push(CompletionItem {
                label: "map: ".to_owned(),
                kind: Some(CompletionItemKind::PROPERTY),
                ..Default::default()
            });
        }
    }

    fn datasource_completions(&self, config: &crate::Configuration, completion_list: &mut CompletionList) {
        let ds = match config.datasources.first() {
            Some(ds) => ds,
            None => return,
        };

        if !ds.schemas_defined() {
            completions::schemas_completion(completion_list);
        }
    }

    fn flavour(&self) -> Flavour {
        Flavour::Sqlserver
    }

    fn does_manage_udts(&self) -> bool {
        true
    }

    fn can_assume_strict_equality_in_joins(&self) -> bool {
        true
    }
}

/// A collection of types stored outside of the row to the heap, having
/// certain properties such as not allowed in keys or normal indices.
pub(crate) fn heap_allocated_types() -> &'static [MsSqlType] {
    &[
        Text,
        NText,
        Image,
        Xml,
        VarBinary(Some(Max)),
        VarChar(Some(Max)),
        NVarChar(Some(Max)),
    ]
}
