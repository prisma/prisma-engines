mod datasource;
mod native_types;
mod validations;

pub use native_types::PostgresType;

use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, InsertTextFormat};
use psl_core::{
    datamodel_connector::{
        Connector, ConnectorCapability, ConstraintScope, NativeTypeConstructor, NativeTypeInstance, RelationMode,
        StringFilter,
    },
    diagnostics::Diagnostics,
    parser_database::{ast, walkers, IndexAlgorithm, OperatorClass, ParserDatabase, ReferentialAction, ScalarType},
    Configuration, Datasource, DatasourceConnectorData, PreviewFeature,
};
use std::{borrow::Cow, collections::HashMap};
use PostgresType::*;

use crate::completions;

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[
    ConstraintScope::GlobalPrimaryKeyKeyIndex,
    ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey,
];

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
    ConnectorCapability::EnumArrayPush,
    ConnectorCapability::FullTextSearchWithoutIndex,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::Json,
    ConnectorCapability::JsonFiltering,
    ConnectorCapability::JsonFilteringArrayPath,
    ConnectorCapability::JsonFilteringAlphanumeric,
    ConnectorCapability::JsonFilteringAlphanumericFieldRef,
    ConnectorCapability::MultiSchema,
    ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::NamedPrimaryKeys,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::JsonLists,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::WritableAutoincField,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::OrderByNullsFirstLast,
    ConnectorCapability::SupportsTxIsolationReadUncommitted,
    ConnectorCapability::SupportsTxIsolationReadCommitted,
    ConnectorCapability::SupportsTxIsolationRepeatableRead,
    ConnectorCapability::SupportsTxIsolationSerializable,
    ConnectorCapability::NativeUpsert,
];

pub struct PostgresDatamodelConnector;

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, PostgresType)] = &[
    (ScalarType::Int, PostgresType::Integer),
    (ScalarType::BigInt, PostgresType::BigInt),
    (ScalarType::Float, PostgresType::DoublePrecision),
    (ScalarType::Decimal, PostgresType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, PostgresType::Boolean),
    (ScalarType::String, PostgresType::Text),
    (ScalarType::DateTime, PostgresType::Timestamp(Some(3))),
    (ScalarType::Bytes, PostgresType::ByteA),
    (ScalarType::Json, PostgresType::JsonB),
];

/// Postgres-specific properties in the datasource block.
#[derive(Default, Debug)]
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

    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | Cascade
    }

    /// Postgres accepts table definitions with a SET NULL referential action referencing a non-nullable field,
    /// although that would lead to a runtime error once the action is actually triggered.
    fn allows_set_null_referential_action_on_non_nullable_fields(&self, relation_mode: RelationMode) -> bool {
        relation_mode.uses_foreign_keys()
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        let native_type: &PostgresType = native_type.downcast_ref();

        match native_type {
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
            Money => ScalarType::Float,
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
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> NativeTypeInstance {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {scalar_type:?} in SCALAR_TYPE_DEFAULTS"))
            .unwrap();

        NativeTypeInstance::new::<PostgresType>(*native_type)
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let native_type: &PostgresType = native_type.downcast_ref();

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: ast::Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &PostgresType = native_type_instance.downcast_ref();
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
        let native_type = PostgresType::from_parts(name, args, span, diagnostics)?;
        Some(NativeTypeInstance::new::<PostgresType>(native_type))
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
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
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
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
                    ast::AttributePosition::FunctionArgument(field_name, "ops"),
                ),
            ) => {
                // let's not care about composite field indices yet
                if field_name.contains('.') {
                    return;
                }

                let index_field = db
                    .walk_models()
                    .chain(db.walk_views())
                    .find(|model| model.model_id() == model_id)
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

        if config.preview_features().contains(PreviewFeature::MultiSchema) && !ds.schemas_defined() {
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

        DatasourceConnectorData::new(Box::new(properties))
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
