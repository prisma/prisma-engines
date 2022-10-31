mod native_types;
mod validations;

pub use native_types::CockroachType;

use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};
use psl_core::{
    datamodel_connector::{
        Connector, ConnectorCapability, ConstraintScope, NativeTypeConstructor, NativeTypeInstance, StringFilter,
    },
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{
        self,
        ast::{self, SchemaPosition},
        coerce,
        walkers::ModelWalker,
        IndexAlgorithm, ParserDatabase, ReferentialAction, ScalarType,
    },
};
use std::borrow::Cow;

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey];

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::AdvancedJsonNullability,
    ConnectorCapability::AnyId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::AutoIncrementAllowedOnNonId,
    ConnectorCapability::AutoIncrementMultipleAllowed,
    ConnectorCapability::AutoIncrementNonIndexedAllowed,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::CreateMany,
    ConnectorCapability::CreateManyWriteableAutoIncId,
    ConnectorCapability::CreateSkipDuplicates,
    ConnectorCapability::Enums,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::Json,
    ConnectorCapability::JsonFiltering,
    ConnectorCapability::JsonFilteringArrayPath,
    ConnectorCapability::NamedPrimaryKeys,
    ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::WritableAutoincField,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::OrderByNullsFirstLast,
    ConnectorCapability::SupportsTxIsolationSerializable,
    ConnectorCapability::NativeUpsert,
];

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, CockroachType)] = &[
    (ScalarType::Int, CockroachType::Int4),
    (ScalarType::BigInt, CockroachType::Int8),
    (ScalarType::Float, CockroachType::Float8),
    (ScalarType::Decimal, CockroachType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, CockroachType::Bool),
    (ScalarType::String, CockroachType::String(None)),
    (ScalarType::DateTime, CockroachType::Timestamp(Some(3))),
    (ScalarType::Bytes, CockroachType::Bytes),
    (ScalarType::Json, CockroachType::JsonB),
];

pub(crate) struct CockroachDatamodelConnector;

impl Connector for CockroachDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "cockroachdb"
    }

    fn name(&self) -> &str {
        "CockroachDB"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    /// The maximum length of postgres identifiers, in bytes.
    ///
    /// Reference: <https://www.postgresql.org/docs/12/limits.html>
    fn max_identifier_length(&self) -> usize {
        63
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        NoAction | Restrict | Cascade | SetNull | SetDefault
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        let native_type: &CockroachType = native_type.downcast_ref();

        match native_type {
            // String
            CockroachType::Char(_) => ScalarType::String,
            CockroachType::CatalogSingleChar => ScalarType::String,
            CockroachType::String(_) => ScalarType::String,
            CockroachType::Bit(_) => ScalarType::String,
            CockroachType::VarBit(_) => ScalarType::String,
            CockroachType::Uuid => ScalarType::String,
            CockroachType::Inet => ScalarType::String,
            // Boolean
            CockroachType::Bool => ScalarType::Boolean,
            // Int
            CockroachType::Int2 => ScalarType::Int,
            CockroachType::Int4 => ScalarType::Int,
            CockroachType::Oid => ScalarType::Int,
            // BigInt
            CockroachType::Int8 => ScalarType::BigInt,
            // Float
            CockroachType::Float4 => ScalarType::Float,
            CockroachType::Float8 => ScalarType::Float,
            // Decimal
            CockroachType::Decimal(_) => ScalarType::Decimal,
            // DateTime
            CockroachType::Timestamp(_) => ScalarType::DateTime,
            CockroachType::Timestamptz(_) => ScalarType::DateTime,
            CockroachType::Date => ScalarType::DateTime,
            CockroachType::Time(_) => ScalarType::DateTime,
            CockroachType::Timetz(_) => ScalarType::DateTime,
            // Json
            CockroachType::JsonB => ScalarType::Json,
            // Bytes
            CockroachType::Bytes => ScalarType::Bytes,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> NativeTypeInstance {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {:?} in SCALAR_TYPE_DEFAULTS", scalar_type))
            .unwrap();

        NativeTypeInstance::new::<CockroachType>(*native_type)
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let native_type: &CockroachType = native_type.downcast_ref();

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && native_type == nt)
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        native_type.downcast_ref::<CockroachType>().to_parts()
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: ast::Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &CockroachType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            CockroachType::Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            CockroachType::Decimal(Some((prec, _))) if *prec > 1000 || *prec == 0 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    "Precision must be positive with a maximum value of 1000.",
                    span,
                ))
            }
            CockroachType::Bit(Some(0)) | CockroachType::VarBit(Some(0)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("M must be a positive integer.", span))
            }
            CockroachType::Timestamp(Some(p))
            | CockroachType::Timestamptz(Some(p))
            | CockroachType::Time(Some(p))
            | CockroachType::Timetz(Some(p))
                if *p > 6 =>
            {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 6.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, model: ModelWalker<'_>, diagnostics: &mut Diagnostics) {
        validations::autoincrement_validations(model, diagnostics);

        for index in model.indexes() {
            validations::inverted_index_validations(index, diagnostics);
        }
    }

    fn validate_scalar_field_unknown_default_functions(
        &self,
        db: &parser_database::ParserDatabase,
        diagnostics: &mut Diagnostics,
    ) {
        for d in db.walk_scalar_field_defaults_with_unknown_function() {
            let (func_name, args, span) = d.value().as_function().unwrap();
            match func_name {
                "sequence" => {
                    SequenceFunction::validate(args, diagnostics);
                }
                _ => diagnostics.push_error(DatamodelError::new_default_unknown_function(func_name, span)),
            }
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        native_types::CONSTRUCTORS
    }

    fn supported_index_types(&self) -> BitFlags<IndexAlgorithm> {
        BitFlags::empty() | IndexAlgorithm::BTree | IndexAlgorithm::Gin
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: ast::Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        let native_type = CockroachType::from_parts(name, args, span, diagnostics)?;
        Some(NativeTypeInstance::new::<CockroachType>(native_type))
    }

    fn scalar_filter_name(&self, scalar_type_name: String, native_type_name: Option<&str>) -> Cow<'_, str> {
        match native_type_name {
            Some(name) if name.eq_ignore_ascii_case("uuid") => "Uuid".into(),
            _ => scalar_type_name.into(),
        }
    }

    fn string_filters(&self, input_object_name: &str) -> BitFlags<StringFilter> {
        match input_object_name {
            "Uuid" => BitFlags::empty(),
            _ => BitFlags::all(),
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err("must start with the protocol `postgresql://` or `postgres://`.".to_owned());
        }

        Ok(())
    }

    fn push_completions(&self, _db: &ParserDatabase, position: SchemaPosition<'_>, completions: &mut CompletionList) {
        if let ast::SchemaPosition::Model(
            _,
            ast::ModelPosition::ModelAttribute("index", _, ast::AttributePosition::Argument("type")),
        ) = position
        {
            for index_type in self.supported_index_types() {
                completions.items.push(CompletionItem {
                    label: index_type.to_string(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some(index_type.documentation().to_owned()),
                    ..Default::default()
                });
            }
        }
    }
}

/// An `@default(sequence())` function.
#[derive(Default, Debug)]
pub struct SequenceFunction {
    pub r#virtual: Option<bool>,
    pub cache: Option<i64>,
    pub increment: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub start: Option<i64>,
}

impl SequenceFunction {
    pub fn build_unchecked(args: &ast::ArgumentsList) -> Self {
        Self::validate(args, &mut Diagnostics::default())
    }

    pub fn validate(args: &ast::ArgumentsList, diagnostics: &mut Diagnostics) -> Self {
        let mut this = SequenceFunction::default();

        for arg in &args.arguments {
            match arg.name.as_ref().map(|arg| arg.name.as_str()) {
                Some("virtual") => this.r#virtual = coerce::boolean(&arg.value, diagnostics),
                Some("cache") => this.cache = coerce::integer(&arg.value, diagnostics),
                Some("increment") => this.increment = coerce::integer(&arg.value, diagnostics),
                Some("minValue") => this.min_value = coerce::integer(&arg.value, diagnostics),
                Some("maxValue") => this.max_value = coerce::integer(&arg.value, diagnostics),
                Some("start") => this.start = coerce::integer(&arg.value, diagnostics),
                Some(_) | None => diagnostics.push_error(DatamodelError::new_static(
                    "Unexpected argument in `sequence()` function call",
                    arg.span,
                )),
            }
        }

        this
    }
}
