mod datasource;
mod native_types;
mod validations;

pub use native_types::{KnownPostgresType, PostgresType};
use parser_database::{ExtensionTypes, ScalarFieldType};

use crate::{
    Configuration, Datasource, DatasourceConnectorData, PreviewFeature, ValidatedSchema,
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance, NativeTypeParseError, RelationMode, StringFilter,
    },
    diagnostics::Diagnostics,
    parser_database::{IndexAlgorithm, OperatorClass, ParserDatabase, ReferentialAction, ScalarType, ast, walkers},
};
use KnownPostgresType::*;
use chrono::*;
use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, InsertTextFormat};
use std::{borrow::Cow, collections::HashMap, sync::Arc};

use super::completions;

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[
    ConstraintScope::GlobalPrimaryKeyKeyIndex,
    ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey,
];

pub const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    AdvancedJsonNullability |
    AnyId |
    AutoIncrement |
    AutoIncrementAllowedOnNonId |
    AutoIncrementMultipleAllowed |
    AutoIncrementNonIndexedAllowed |
    CompoundIds |
    CreateMany |
    CreateManyWriteableAutoIncId |
    CreateSkipDuplicates |
    Enums |
    EnumArrayPush |
    NativeFullTextSearch |
    NativeFullTextSearchWithoutIndex |
    InsensitiveFilters |
    Json |
    JsonFiltering |
    JsonFilteringArrayPath |
    JsonFilteringAlphanumeric |
    JsonFilteringAlphanumericFieldRef |
    JsonArrayContains |
    MultiSchema |
    NamedForeignKeys |
    NamedPrimaryKeys |
    RelationFieldsInArbitraryOrder |
    ScalarLists |
    JsonLists |
    UpdateableId |
    WritableAutoincField |
    ImplicitManyToManyRelation |
    DecimalType |
    OrderByNullsFirstLast |
    FilteredInlineChildNestedToOneDisconnect |
    SupportsTxIsolationReadUncommitted |
    SupportsTxIsolationReadCommitted |
    SupportsTxIsolationRepeatableRead |
    SupportsTxIsolationSerializable |
    NativeUpsert |
    InsertReturning |
    UpdateReturning |
    RowIn |
    DistinctOn |
    DeleteReturning |
    SupportsFiltersOnRelationsWithoutJoins |
    LateralJoin |
    SupportsDefaultInInsert |
    PartialIndex
});

pub struct PostgresDatamodelConnector;

const DATE_TIME_DEFAULT: KnownPostgresType = KnownPostgresType::Timestamp(Some(3));
const BYTES_DEFAULT: KnownPostgresType = KnownPostgresType::ByteA;

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, KnownPostgresType)] = &[
    (ScalarType::Int, KnownPostgresType::Integer),
    (ScalarType::BigInt, KnownPostgresType::BigInt),
    (ScalarType::Float, KnownPostgresType::DoublePrecision),
    (ScalarType::Decimal, KnownPostgresType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, KnownPostgresType::Boolean),
    (ScalarType::String, KnownPostgresType::Text),
    (ScalarType::DateTime, DATE_TIME_DEFAULT),
    (ScalarType::Bytes, BYTES_DEFAULT),
    (ScalarType::Json, KnownPostgresType::JsonB),
];

/// Postgres-specific properties in the datasource block.
#[derive(Default, Debug, Clone)]
pub struct PostgresDatasourceProperties {
    extensions: Option<PostgresExtensions>,
}

impl PostgresDatasourceProperties {
    /// Database extensions.
    pub fn extensions(&self) -> Option<&PostgresExtensions> {
        self.extensions.as_ref()
    }

    pub fn set_extensions(&mut self, extensions: Vec<PostgresExtension>) {
        self.extensions = Some(PostgresExtensions {
            extensions,
            span: ast::Span::empty(),
        });
    }

    // Validation for property existence
    pub fn extensions_defined(&self) -> bool {
        self.extensions.is_some()
    }
}

/// An extension defined in the extensions array of the datasource.
///
/// ```ignore
/// datasource db {
///   extensions = [postgis, foobar]
///   //            ^^^^^^^
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PostgresExtension {
    name: String,
    span: ast::Span,
    schema: Option<String>,
    version: Option<String>,
    db_name: Option<String>,
}

impl PostgresExtension {
    pub fn new(name: String) -> Self {
        Self {
            name,
            span: ast::Span::empty(),
            schema: None,
            version: None,
            db_name: None,
        }
    }

    pub fn set_span(&mut self, span: ast::Span) {
        self.span = span;
    }

    pub fn set_schema(&mut self, schema: String) {
        self.schema = Some(schema);
    }

    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    pub fn set_db_name(&mut self, db_name: String) {
        self.db_name = Some(db_name);
    }

    /// The name of the extension in the datasource.
    ///
    /// ```ignore
    /// extensions = [bar]
    /// //            ^^^ this
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the extension in the database, defined in the
    /// `map` argument.
    ///
    /// ```ignore
    /// extensions = [bar(map: "foo")]
    /// //                     ^^^^^ this
    /// ```
    pub fn db_name(&self) -> Option<&str> {
        self.db_name.as_deref()
    }

    /// The span of the extension definition in the datamodel.
    pub fn span(&self) -> ast::Span {
        self.span
    }

    /// The schema where the extension tables are stored.
    ///
    /// ```ignore
    /// extensions = [postgis(schema: "public")]
    /// //                            ^^^^^^^^ this
    /// ```
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// The version of the extension to be used in the database.
    ///
    /// ```ignore
    /// extensions = [postgis(version: "2.1")]
    /// //                             ^^^^^ this
    /// ```
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// The extensions defined in the extensions array of the datrasource.
///
/// ```ignore
/// datasource db {
///   extensions = [postgis, foobar]
///   //           ^^^^^^^^^^^^^^^^^
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PostgresExtensions {
    pub(crate) extensions: Vec<PostgresExtension>,
    pub(crate) span: ast::Span,
}

impl Default for PostgresExtensions {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            span: ast::Span::empty(),
        }
    }
}

impl PostgresExtensions {
    /// The span of the extensions in the datamodel.
    pub fn span(&self) -> ast::Span {
        self.span
    }

    /// The extension definitions.
    pub fn extensions(&self) -> &[PostgresExtension] {
        &self.extensions
    }

    /// Finds the extension with the given database name.
    pub fn find_by_name(&self, name: &str) -> Option<&PostgresExtension> {
        self.extensions().iter().find(|ext| {
            ext.db_name()
                .map(|db_name| db_name == name)
                .unwrap_or_else(|| ext.name() == name)
        })
    }
}

impl Connector for PostgresDatamodelConnector {
    fn is_provider(&self, name: &str) -> bool {
        ["postgresql", "postgres"].contains(&name)
    }

    fn provider_name(&self) -> &'static str {
        "postgresql"
    }

    fn name(&self) -> &str {
        "Postgres"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    /// The connector-specific name of the `fullTextSearch` preview feature.
    fn native_full_text_search_preview_feature(&self) -> Option<PreviewFeature> {
        Some(PreviewFeature::FullTextSearchPostgres)
    }

    /// The maximum length of postgres identifiers, in bytes.
    ///
    /// Reference: <https://www.postgresql.org/docs/12/limits.html>
    fn max_identifier_length(&self) -> usize {
        63
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        NoAction | Restrict | Cascade | SetNull | SetDefault
    }

    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | Cascade
    }

    /// Postgres accepts table definitions with a SET NULL referential action referencing a non-nullable field,
    /// although that would lead to a runtime error once the action is actually triggered.
    fn allows_set_null_referential_action_on_non_nullable_fields(&self, relation_mode: RelationMode) -> bool {
        relation_mode.uses_foreign_keys()
    }

    fn scalar_type_for_native_type(
        &self,
        native_type: &NativeTypeInstance,
        extension_types: &dyn ExtensionTypes,
    ) -> Option<ScalarFieldType> {
        let native_type = match native_type.downcast_ref() {
            PostgresType::Known(st) => st,
            PostgresType::Unknown(name, modifiers) => {
                return extension_types
                    .get_by_db_name_and_modifiers(name, Some(modifiers))
                    .map(|e| ScalarFieldType::Extension(e.id));
            }
        };

        let res = match native_type {
            // String
            Text => ScalarType::String,
            Char(_) => ScalarType::String,
            VarChar(_) => ScalarType::String,
            Bit(_) => ScalarType::String,
            VarBit(_) => ScalarType::String,
            Uuid => ScalarType::String,
            Xml => ScalarType::String,
            Inet => ScalarType::String,
            Citext => ScalarType::String,
            // Boolean
            Boolean => ScalarType::Boolean,
            // Int
            SmallInt => ScalarType::Int,
            Integer => ScalarType::Int,
            Oid => ScalarType::Int,
            // BigInt
            BigInt => ScalarType::BigInt,
            // Float
            Real => ScalarType::Float,
            DoublePrecision => ScalarType::Float,
            // Decimal
            Decimal(_) => ScalarType::Decimal,
            Money => ScalarType::Decimal,
            // DateTime
            Timestamp(_) => ScalarType::DateTime,
            Timestamptz(_) => ScalarType::DateTime,
            Date => ScalarType::DateTime,
            Time(_) => ScalarType::DateTime,
            Timetz(_) => ScalarType::DateTime,
            // Json
            Json => ScalarType::Json,
            JsonB => ScalarType::Json,
            // Bytes
            ByteA => ScalarType::Bytes,
        };
        Some(ScalarFieldType::BuiltInScalar(res))
    }

    fn default_native_type_for_scalar_type(
        &self,
        scalar_type: &ScalarFieldType,
        schema: &ValidatedSchema,
    ) -> Option<NativeTypeInstance> {
        let native_type = match scalar_type {
            ScalarFieldType::BuiltInScalar(scalar_type) => PostgresType::Known(
                *SCALAR_TYPE_DEFAULTS
                    .iter()
                    .find(|(st, _)| st == scalar_type)
                    .map(|(_, native_type)| native_type)
                    .ok_or_else(|| format!("Could not find scalar type {scalar_type:?} in SCALAR_TYPE_DEFAULTS"))
                    .unwrap(),
            ),
            ScalarFieldType::Extension(id) => {
                let (name, modifiers) = schema.db.get_extension_type_db_name_with_modifiers(*id)?;
                let native_type = PostgresType::Unknown(name.to_owned(), modifiers.to_vec());
                return Some(NativeTypeInstance::new::<PostgresType>(native_type));
            }
            ScalarFieldType::CompositeType(_) | ScalarFieldType::Enum(_) | ScalarFieldType::Unsupported(_) => {
                return None;
            }
        };

        Some(NativeTypeInstance::new::<PostgresType>(native_type))
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: Option<ScalarType>,
        span: ast::Span,
        errors: &mut Diagnostics,
    ) {
        let PostgresType::Known(native_type) = native_type_instance.downcast_ref() else {
            return;
        };
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            Decimal(Some((prec, _))) if *prec > 1000 || *prec == 0 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    "Precision must be positive with a maximum value of 1000.",
                    span,
                ))
            }
            Bit(Some(0)) | VarBit(Some(0)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("M must be a positive integer.", span))
            }
            Timestamp(Some(p)) | Timestamptz(Some(p)) | Time(Some(p)) | Timetz(Some(p)) if *p > 6 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 6.", span))
            }
            _ => (),
        }
    }

    fn native_type_supports_compacting(&self, nt: Option<NativeTypeInstance>) -> bool {
        let native_type: Option<&PostgresType> = nt.as_ref().map(|nt| nt.downcast_ref());

        match native_type {
            Some(pt) => !matches!(pt, PostgresType::Known(Citext)),
            None => true,
        }
    }

    fn validate_model(&self, model: walkers::ModelWalker<'_>, _: RelationMode, errors: &mut Diagnostics) {
        for index in model.indexes() {
            validations::compatible_native_types(index, self, errors);
            validations::generalized_index_validations(index, self, errors);
            validations::spgist_indexed_column_count(index, errors);
        }
    }

    fn validate_datasource(
        &self,
        preview_features: BitFlags<PreviewFeature>,
        ds: &Datasource,
        errors: &mut Diagnostics,
    ) {
        if let Some(props) = ds.downcast_connector_data::<PostgresDatasourceProperties>() {
            validations::extensions_preview_flag_must_be_set(preview_features, props, errors);
            validations::extension_names_follow_prisma_syntax_rules(preview_features, props, errors);
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        native_types::CONSTRUCTORS
    }

    fn supported_index_types(&self) -> BitFlags<IndexAlgorithm> {
        BitFlags::empty()
            | IndexAlgorithm::BTree
            | IndexAlgorithm::Gist
            | IndexAlgorithm::Hash
            | IndexAlgorithm::Gin
            | IndexAlgorithm::SpGist
            | IndexAlgorithm::Brin
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: ast::Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        let nt = match KnownPostgresType::from_parts(name, args) {
            Ok(res) => PostgresType::Known(res),
            Err(NativeTypeParseError::UnknownType { .. }) => PostgresType::Unknown(name.to_owned(), args.to_owned()),
            Err(err) => {
                diagnostics.push_error(err.into_datamodel_error(span));
                return None;
            }
        };
        Some(NativeTypeInstance::new(nt))
    }

    fn native_type_to_parts<'t>(&self, native_type: &'t NativeTypeInstance) -> (&'t str, Cow<'t, [String]>) {
        native_type.downcast_ref::<PostgresType>().to_parts()
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
        if !url.starts_with("postgres://")
            && !url.starts_with("postgresql://")
            && !url.starts_with("prisma+postgres://")
        {
            return Err("must start with the protocol `postgresql://` or `postgres://`.".to_owned());
        }

        Ok(())
    }

    fn datamodel_completions(
        &self,
        db: &ParserDatabase,
        position: ast::SchemaPosition<'_>,
        completions: &mut CompletionList,
    ) {
        match position {
            ast::SchemaPosition::Model(
                _,
                ast::ModelPosition::ModelAttribute("index", _, ast::AttributePosition::Argument("type")),
            ) => {
                for index_type in self.supported_index_types() {
                    completions.items.push(CompletionItem {
                        label: index_type.to_string(),
                        kind: Some(CompletionItemKind::ENUM),
                        detail: Some(index_type.documentation().to_owned()),
                        ..Default::default()
                    });
                }
            }
            ast::SchemaPosition::Model(
                model_id,
                ast::ModelPosition::ModelAttribute(
                    "index",
                    attr_id,
                    ast::AttributePosition::FunctionArgument(field_name, "ops", _),
                ),
            ) => {
                // let's not care about composite field indices yet
                if field_name.contains('.') {
                    return;
                }

                let index_field = db
                    .walk_models()
                    .chain(db.walk_views())
                    .find(|model| model.id.1 == model_id)
                    .and_then(|model| {
                        model.indexes().find(|index| {
                            index.attribute_id()
                                == ast::AttributeId::new_in_container(ast::AttributeContainer::Model(model_id), attr_id)
                        })
                    })
                    .and_then(|index| {
                        index
                            .fields()
                            .find(|f| f.name() == field_name)
                            .and_then(|f| f.as_scalar_field())
                            .map(|field| (index, field))
                    });

                if let Some((index, field)) = index_field {
                    let algo = index.algorithm().unwrap_or_default();

                    for operator in allowed_index_operator_classes(algo, field) {
                        completions.items.push(CompletionItem {
                            label: operator.to_string(),
                            kind: Some(CompletionItemKind::ENUM),
                            ..Default::default()
                        });
                    }

                    completions.items.push(CompletionItem {
                        label: "raw".to_string(),
                        insert_text: Some(r#"raw("$0")"#.to_string()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        kind: Some(CompletionItemKind::FUNCTION),
                        ..Default::default()
                    });
                }
            }
            _ => (),
        }
    }

    fn datasource_completions(&self, config: &Configuration, completion_list: &mut CompletionList) {
        let ds = match config.datasources.first() {
            Some(ds) => ds,
            None => return,
        };

        let connector_data = ds
            .connector_data
            .downcast_ref::<PostgresDatasourceProperties>()
            .unwrap();

        if config.preview_features().contains(PreviewFeature::PostgresqlExtensions)
            && !connector_data.extensions_defined()
        {
            completions::extensions_completion(completion_list);
        }

        if !ds.schemas_defined() {
            completions::schemas_completion(completion_list);
        }
    }

    fn parse_datasource_properties(
        &self,
        args: &mut HashMap<&str, (ast::Span, &ast::Expression)>,
        diagnostics: &mut Diagnostics,
    ) -> DatasourceConnectorData {
        let extensions = datasource::parse_extensions(args, diagnostics);
        let properties = PostgresDatasourceProperties { extensions };

        DatasourceConnectorData::new(Arc::new(properties))
    }

    fn flavour(&self) -> Flavour {
        Flavour::Postgres
    }

    fn parse_json_datetime(
        &self,
        str: &str,
        nt: Option<NativeTypeInstance>,
    ) -> chrono::ParseResult<chrono::DateTime<FixedOffset>> {
        let native_type = nt.as_ref().and_then(|nt| nt.downcast_ref::<PostgresType>().as_known());

        match native_type {
            Some(pt) => match pt {
                Timestamptz(_) => super::utils::postgres::parse_timestamptz(str),
                Timestamp(_) => super::utils::postgres::parse_timestamp(str),
                Date => super::utils::common::parse_date(str),
                Time(_) => super::utils::common::parse_time(str),
                Timetz(_) => super::utils::postgres::parse_timetz(str),
                _ => unreachable!(),
            },
            None => self.parse_json_datetime(
                str,
                Some(NativeTypeInstance::new(PostgresType::Known(DATE_TIME_DEFAULT))),
            ),
        }
    }

    fn parse_json_bytes(&self, str: &str, nt: Option<NativeTypeInstance>) -> prisma_value::PrismaValueResult<Vec<u8>> {
        let native_type = nt.as_ref().and_then(|nt| nt.downcast_ref::<PostgresType>().as_known());

        match native_type {
            Some(ct) => match ct {
                KnownPostgresType::ByteA => {
                    super::utils::postgres::parse_bytes(str).map_err(|_| prisma_value::ConversionFailure {
                        from: "hex".into(),
                        to: "bytes".into(),
                    })
                }
                _ => unreachable!(),
            },
            None => self.parse_json_bytes(str, Some(NativeTypeInstance::new(PostgresType::Known(BYTES_DEFAULT)))),
        }
    }

    fn can_assume_strict_equality_in_joins(&self) -> bool {
        true
    }
}

fn allowed_index_operator_classes(algo: IndexAlgorithm, field: walkers::ScalarFieldWalker<'_>) -> Vec<OperatorClass> {
    let scalar_type = field.scalar_type();
    let native_type = field.raw_native_type().map(|t| t.1);

    let mut classes = Vec::new();

    match (algo, scalar_type, native_type) {
        (IndexAlgorithm::Gist, _, Some("Inet")) => {
            classes.push(OperatorClass::InetOps);
        }

        (IndexAlgorithm::Gin, _, _) if field.ast_field().arity.is_list() => {
            classes.push(OperatorClass::ArrayOps);
        }
        (IndexAlgorithm::Gin, Some(ScalarType::Json), _) => {
            classes.push(OperatorClass::JsonbOps);
            classes.push(OperatorClass::JsonbPathOps);
        }

        (IndexAlgorithm::SpGist, _, Some("Inet")) => {
            classes.push(OperatorClass::InetOps);
        }
        (IndexAlgorithm::SpGist, Some(ScalarType::String), None | Some("Text") | Some("VarChar")) => {
            classes.push(OperatorClass::TextOps);
        }

        (IndexAlgorithm::Brin, _, Some("Bit")) => {
            classes.push(OperatorClass::BitMinMaxOps);
        }
        (IndexAlgorithm::Brin, _, Some("VarBit")) => {
            classes.push(OperatorClass::VarBitMinMaxOps);
        }
        (IndexAlgorithm::Brin, _, Some("Char")) => {
            classes.push(OperatorClass::BpcharBloomOps);
            classes.push(OperatorClass::BpcharMinMaxOps);
        }
        (IndexAlgorithm::Brin, _, Some("Date")) => {
            classes.push(OperatorClass::DateBloomOps);
            classes.push(OperatorClass::DateMinMaxOps);
            classes.push(OperatorClass::DateMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Real")) => {
            classes.push(OperatorClass::Float4BloomOps);
            classes.push(OperatorClass::Float4MinMaxOps);
            classes.push(OperatorClass::Float4MinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::Float), _) => {
            classes.push(OperatorClass::Float8BloomOps);
            classes.push(OperatorClass::Float8MinMaxOps);
            classes.push(OperatorClass::Float8MinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Inet")) => {
            classes.push(OperatorClass::InetBloomOps);
            classes.push(OperatorClass::InetInclusionOps);
            classes.push(OperatorClass::InetMinMaxOps);
            classes.push(OperatorClass::InetMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("SmallInt")) => {
            classes.push(OperatorClass::Int2BloomOps);
            classes.push(OperatorClass::Int2MinMaxOps);
            classes.push(OperatorClass::Int2MinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::Int), None | Some("Integer")) => {
            classes.push(OperatorClass::Int4BloomOps);
            classes.push(OperatorClass::Int4MinMaxOps);
            classes.push(OperatorClass::Int4MinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::BigInt), _) => {
            classes.push(OperatorClass::Int8BloomOps);
            classes.push(OperatorClass::Int8MinMaxOps);
            classes.push(OperatorClass::Int8MinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::Decimal), _) => {
            classes.push(OperatorClass::NumericBloomOps);
            classes.push(OperatorClass::NumericMinMaxOps);
            classes.push(OperatorClass::NumericMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Oid")) => {
            classes.push(OperatorClass::OidBloomOps);
            classes.push(OperatorClass::OidMinMaxOps);
            classes.push(OperatorClass::OidMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::Bytes), None | Some("ByteA")) => {
            classes.push(OperatorClass::ByteaBloomOps);
            classes.push(OperatorClass::ByteaMinMaxOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::String), None | Some("Text") | Some("VarChar")) => {
            classes.push(OperatorClass::TextBloomOps);
            classes.push(OperatorClass::TextMinMaxOps);
        }
        (IndexAlgorithm::Brin, Some(ScalarType::DateTime), None | Some("Timestamp")) => {
            classes.push(OperatorClass::TimestampBloomOps);
            classes.push(OperatorClass::TimestampMinMaxOps);
            classes.push(OperatorClass::TimestampMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Timestamptz")) => {
            classes.push(OperatorClass::TimestampTzBloomOps);
            classes.push(OperatorClass::TimestampTzMinMaxOps);
            classes.push(OperatorClass::TimestampTzMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Time")) => {
            classes.push(OperatorClass::TimeBloomOps);
            classes.push(OperatorClass::TimeMinMaxOps);
            classes.push(OperatorClass::TimeMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Timetz")) => {
            classes.push(OperatorClass::TimeTzBloomOps);
            classes.push(OperatorClass::TimeTzMinMaxOps);
            classes.push(OperatorClass::TimeTzMinMaxMultiOps);
        }
        (IndexAlgorithm::Brin, _, Some("Uuid")) => {
            classes.push(OperatorClass::UuidBloomOps);
            classes.push(OperatorClass::UuidMinMaxOps);
            classes.push(OperatorClass::UuidMinMaxMultiOps);
        }

        _ => (),
    }

    classes
}
