//! The interface implemented by connectors for Prisma schema validation and interpretation.

/// Connector capabilities
pub mod capabilities;
/// Constraint name defaults.
pub mod constraint_names;
/// Extensions for parser database walkers with context from the connector.
pub mod walker_ext_traits;

/// Connector completions
pub mod completions;

mod empty_connector;
mod filters;
mod native_types;
mod relation_mode;

pub use self::{
    capabilities::{ConnectorCapabilities, ConnectorCapability},
    completions::format_completion_docs,
    empty_connector::EmptyDatamodelConnector,
    filters::*,
    native_types::{NativeTypeArguments, NativeTypeConstructor, NativeTypeInstance},
    relation_mode::RelationMode,
};

use crate::{configuration::DatasourceConnectorData, Configuration, Datasource, PreviewFeature};
use diagnostics::{DatamodelError, Diagnostics, NativeTypeErrorFactory, Span};
use enumflags2::BitFlags;
use lsp_types::CompletionList;
use parser_database::{
    ast::{self, SchemaPosition},
    walkers, IndexAlgorithm, ParserDatabase, ReferentialAction, ScalarType,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

pub const EXTENSIONS_KEY: &str = "extensions";

/// The datamodel connector API.
pub trait Connector: Send + Sync {
    /// The name of the provider, for string comparisons determining which connector we are on.
    fn provider_name(&self) -> &'static str;

    /// Must return true whenever the passed in provider name is a match.
    fn is_provider(&self, name: &str) -> bool {
        name == self.provider_name()
    }

    /// The name of the connector. Can be used in error messages.
    fn name(&self) -> &str;

    /// The static list of capabilities for the connector.
    fn capabilities(&self) -> &'static [ConnectorCapability];

    /// Does the connector have this capability?
    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    /// The maximum length of constraint names in bytes. Connectors without a
    /// limit should return usize::MAX.
    fn max_identifier_length(&self) -> usize;

    // Relation mode

    /// The relation modes that can be set through the relationMode datasource
    /// argument.
    fn allowed_relation_mode_settings(&self) -> BitFlags<RelationMode> {
        use RelationMode::*;

        ForeignKeys | Prisma
    }

    /// The default relation mode to assume for this connector.
    fn default_relation_mode(&self) -> RelationMode {
        RelationMode::ForeignKeys
    }

    /// The referential actions supported by the connector.
    fn referential_actions(&self) -> BitFlags<ReferentialAction>;

    /// The referential actions supported when using relationMode = "prisma" by the connector.
    /// There are in fact scenarios in which the set of emulated referential actions supported may change
    /// depending on the connector. For example, Postgres' NoAction mode behaves similarly to Restrict
    /// (raising an error if any referencing rows still exist when the constraint is checked), but with
    /// a subtle twist we decided not to emulate: NO ACTION allows the check to be deferred until later
    /// in the transaction, whereas RESTRICT does not.
    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        RelationMode::allowed_emulated_referential_actions_default()
    }

    /// Most SQL databases reject table definitions with a SET NULL referential action referencing a non-nullable field,
    /// but that's not true for all of them.
    /// This was introduced because Postgres accepts data definition language statements with the SET NULL
    /// referential action referencing non-nullable fields, although this would lead to a runtime error once
    /// the action is actually triggered.
    fn allows_set_null_referential_action_on_non_nullable_fields(&self, _relation_mode: RelationMode) -> bool {
        false
    }

    fn supports_composite_types(&self) -> bool {
        self.has_capability(ConnectorCapability::CompositeTypes)
    }

    fn supports_named_primary_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedPrimaryKeys)
    }

    fn supports_named_foreign_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedForeignKeys)
    }

    fn supports_named_default_values(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedDefaultValues)
    }

    fn supports_referential_action(&self, relation_mode: &RelationMode, action: ReferentialAction) -> bool {
        match relation_mode {
            RelationMode::ForeignKeys => self.referential_actions().contains(action),
            RelationMode::Prisma => self.emulated_referential_actions().contains(action),
        }
    }

    /// This is used by the query engine schema builder.
    ///
    /// For a given scalar type + native type combination, this method should return the name to be
    /// given to the filter input objects for the type. The significance of that name is that the
    /// resulting input objects will be cached by name, so for a given filter input object name,
    /// the filters should always be identical.
    fn scalar_filter_name(&self, scalar_type_name: String, _native_type_name: Option<&str>) -> Cow<'_, str> {
        Cow::Owned(scalar_type_name)
    }

    /// This is used by the query engine schema builder. It is only called for filters of String
    /// fields and aggregates.
    ///
    /// For a given filter input object type name returned by `scalar_filter_name`, it should
    /// return the string operations to be made available in the Client API.
    ///
    /// Implementations of this method _must_ always associate the same filters to the same input
    /// object type name. This is because the filter types are cached by name, so if different
    /// calls to the method return different filters, only the first return value will be used.
    fn string_filters(&self, input_object_name: &str) -> BitFlags<StringFilter> {
        match input_object_name {
            "String" => BitFlags::all(), // all the filters are available by default
            _ => panic!("Unexpected scalar input object name for string filters: `{input_object_name}`"),
        }
    }

    /// Validate that the arguments passed to a native type attribute are valid.
    fn validate_native_type_arguments(
        &self,
        _native_type: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        _span: Span,
        _: &mut Diagnostics,
    ) {
    }

    fn validate_enum(&self, _enum: walkers::EnumWalker<'_>, _: &mut Diagnostics) {}
    fn validate_model(&self, _model: walkers::ModelWalker<'_>, _: RelationMode, _: &mut Diagnostics) {}
    fn validate_relation_field(&self, _field: walkers::RelationFieldWalker<'_>, _: &mut Diagnostics) {}
    fn validate_datasource(&self, _: BitFlags<PreviewFeature>, _: &Datasource, _: &mut Diagnostics) {}

    fn validate_scalar_field_unknown_default_functions(
        &self,
        db: &parser_database::ParserDatabase,
        diagnostics: &mut Diagnostics,
    ) {
        for d in db.walk_scalar_field_defaults_with_unknown_function() {
            let (func_name, _, span) = d.value().as_function().unwrap();
            diagnostics.push_error(DatamodelError::new_default_unknown_function(func_name, span));
        }
    }

    /// The scopes in which a constraint name should be validated. If empty, doesn't check for name
    /// clashes in the validation phase.
    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        &[]
    }

    /// Returns all available native type constructors available through this connector.
    /// Powers the auto completion of the VSCode plugin.
    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor];

    /// Returns the default scalar type for the given native type
    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType;

    /// On each connector, each built-in Prisma scalar type (`Boolean`,
    /// `String`, `Float`, etc.) has a corresponding native type.
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> NativeTypeInstance;

    /// Same mapping as `default_native_type_for_scalar_type()`, but in the opposite direction.
    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool;

    /// Debug/error representation of a native type.
    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>);

    fn find_native_type_constructor(&self, name: &str) -> Option<&NativeTypeConstructor> {
        self.available_native_type_constructors()
            .iter()
            .find(|constructor| constructor.name == name)
    }

    /// This function is used during Schema parsing to calculate the concrete native type.
    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance>;

    fn set_config_dir<'a>(&self, config_dir: &std::path::Path, url: &'a str) -> Cow<'a, str> {
        let set_root = |path: &str| {
            let path = std::path::Path::new(path);

            if path.is_relative() {
                Some(config_dir.join(path).to_str().map(ToString::to_string).unwrap())
            } else {
                None
            }
        };

        let mut url = match url::Url::parse(url) {
            Ok(url) => url,
            Err(_) => return Cow::from(url), // bail
        };

        let mut params: BTreeMap<String, String> =
            url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

        url.query_pairs_mut().clear();

        // Only for PostgreSQL + MySQL
        if let Some(path) = params.get("sslcert").map(|s| s.as_str()).and_then(set_root) {
            params.insert("sslcert".into(), path);
        }

        // Only for PostgreSQL + MySQL
        if let Some(path) = params.get("sslidentity").map(|s| s.as_str()).and_then(set_root) {
            params.insert("sslidentity".into(), path);
        }

        // Only for MongoDB
        if let Some(path) = params.get("tlsCAFile").map(|s| s.as_str()).and_then(set_root) {
            params.insert("tlsCAFile".into(), path);
        }

        for (k, v) in params.into_iter() {
            url.query_pairs_mut().append_pair(&k, &v);
        }

        url.to_string().into()
    }

    fn supports_scalar_lists(&self) -> bool {
        self.has_capability(ConnectorCapability::ScalarLists)
    }

    fn supports_enums(&self) -> bool {
        self.has_capability(ConnectorCapability::Enums)
    }

    fn supports_json(&self) -> bool {
        self.has_capability(ConnectorCapability::Json)
    }

    fn supports_json_lists(&self) -> bool {
        self.has_capability(ConnectorCapability::JsonLists)
    }

    fn supports_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrement)
    }

    fn supports_non_id_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementAllowedOnNonId)
    }

    fn supports_multiple_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementMultipleAllowed)
    }

    fn supports_non_indexed_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementNonIndexedAllowed)
    }

    fn supports_compound_ids(&self) -> bool {
        self.has_capability(ConnectorCapability::CompoundIds)
    }

    fn supports_decimal(&self) -> bool {
        self.has_capability(ConnectorCapability::DecimalType)
    }

    fn supported_index_types(&self) -> BitFlags<IndexAlgorithm> {
        IndexAlgorithm::BTree.into()
    }

    fn supports_index_type(&self, algo: IndexAlgorithm) -> bool {
        self.supported_index_types().contains(algo)
    }

    fn allows_relation_fields_in_arbitrary_order(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationFieldsInArbitraryOrder)
    }

    /// If true, the schema validator function checks whether the referencing fields in a `@relation` attribute
    /// are included in an index.
    fn should_suggest_missing_referencing_fields_indexes(&self) -> bool {
        true
    }

    fn native_type_to_string(&self, instance: &NativeTypeInstance) -> String {
        let (name, args) = self.native_type_to_parts(instance);
        let args = if args.is_empty() {
            String::new()
        } else {
            format!("({})", args.join(","))
        };
        format!("{name}{args}")
    }

    fn native_instance_error(&self, instance: &NativeTypeInstance) -> NativeTypeErrorFactory {
        NativeTypeErrorFactory::new(self.native_type_to_string(instance), self.name().to_owned())
    }

    fn validate_url(&self, url: &str) -> Result<(), String>;

    fn datamodel_completions(
        &self,
        _db: &ParserDatabase,
        _position: SchemaPosition<'_>,
        _completions: &mut CompletionList,
    ) {
    }

    fn datasource_completions(&self, _config: &Configuration, _completion_list: &mut CompletionList) {}

    fn parse_datasource_properties(
        &self,
        _args: &mut HashMap<&str, (Span, &ast::Expression)>,
        _diagnostics: &mut Diagnostics,
    ) -> DatasourceConnectorData {
        Default::default()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    KeyOrIdx,
    Default,
}

/// A scope where a constraint name must be unique.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum ConstraintScope {
    /// Globally indices and unique constraints
    GlobalKeyIndex,
    /// Globally foreign keys
    GlobalForeignKey,
    /// Globally primary keys, indices and unique constraints
    GlobalPrimaryKeyKeyIndex,
    /// Globally primary keys, foreign keys and default constraints
    GlobalPrimaryKeyForeignKeyDefault,
    /// Per model indices and unique constraints
    ModelKeyIndex,
    /// Per model primary keys, indices and unique constraints
    ModelPrimaryKeyKeyIndex,
    /// Per model primary keys, foreign keys, indices and unique constraints
    ModelPrimaryKeyKeyIndexForeignKey,
}

impl ConstraintScope {
    /// A beefed-up display for errors.
    pub fn description(self, model_name: &str) -> Cow<'static, str> {
        match self {
            ConstraintScope::GlobalKeyIndex => Cow::from("global for indexes and unique constraints"),
            ConstraintScope::GlobalForeignKey => Cow::from("global for foreign keys"),
            ConstraintScope::GlobalPrimaryKeyKeyIndex => {
                Cow::from("global for primary key, indexes and unique constraints")
            }
            ConstraintScope::GlobalPrimaryKeyForeignKeyDefault => {
                Cow::from("global for primary keys, foreign keys and default constraints")
            }
            ConstraintScope::ModelKeyIndex => {
                Cow::from(format!("on model `{model_name}` for indexes and unique constraints"))
            }
            ConstraintScope::ModelPrimaryKeyKeyIndex => Cow::from(format!(
                "on model `{model_name}` for primary key, indexes and unique constraints"
            )),
            ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey => Cow::from(format!(
                "on model `{model_name}` for primary key, indexes, unique constraints and foreign keys"
            )),
        }
    }
}
