mod validations;

use datamodel_connector::{
    helper::{arg_vec_from_opt, args_vec_from_opt, parse_one_opt_u32, parse_two_opt_u32},
    parser_database::{
        ast,
        walkers::{ModelWalker, ScalarFieldWalker},
        IndexAlgorithm, OperatorClass, ParserDatabase,
    },
    Connector, ConnectorCapability, ConstraintScope, DatamodelError, Diagnostics, NativeTypeConstructor,
    NativeTypeInstance, ReferentialAction, ReferentialIntegrity, ScalarType, Span,
};
use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, InsertTextFormat};
use native_types::{
    NativeType,
    PostgresType::{self, *},
};

const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const INTEGER_TYPE_NAME: &str = "Integer";
const BIG_INT_TYPE_NAME: &str = "BigInt";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const MONEY_TYPE_NAME: &str = "Money";
const INET_TYPE_NAME: &str = "Inet";
const CITEXT_TYPE_NAME: &str = "Citext";
const OID_TYPE_NAME: &str = "Oid";
const REAL_TYPE_NAME: &str = "Real";
const DOUBLE_PRECISION_TYPE_NAME: &str = "DoublePrecision";
const VARCHAR_TYPE_NAME: &str = "VarChar";
const CHAR_TYPE_NAME: &str = "Char";
const TEXT_TYPE_NAME: &str = "Text";
const BYTE_A_TYPE_NAME: &str = "ByteA";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const TIMESTAMP_TZ_TYPE_NAME: &str = "Timestamptz";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const TIME_TZ_TYPE_NAME: &str = "Timetz";
const BOOLEAN_TYPE_NAME: &str = "Boolean";
const BIT_TYPE_NAME: &str = "Bit";
const VAR_BIT_TYPE_NAME: &str = "VarBit";
const UUID_TYPE_NAME: &str = "Uuid";
const XML_TYPE_NAME: &str = "Xml";
const JSON_TYPE_NAME: &str = "Json";
const JSON_B_TYPE_NAME: &str = "JsonB";

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[
    NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(INTEGER_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, &[ScalarType::BigInt]),
    NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, &[ScalarType::Decimal]),
    NativeTypeConstructor::without_args(MONEY_TYPE_NAME, &[ScalarType::Decimal]),
    NativeTypeConstructor::without_args(INET_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(CITEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(OID_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(REAL_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(DOUBLE_PRECISION_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::without_args(TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(BYTE_A_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIMESTAMP_TZ_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(DATE_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIME_TZ_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(BOOLEAN_TYPE_NAME, &[ScalarType::Boolean]),
    NativeTypeConstructor::with_optional_args(BIT_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(VAR_BIT_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::without_args(UUID_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(XML_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(JSON_TYPE_NAME, &[ScalarType::Json]),
    NativeTypeConstructor::without_args(JSON_B_TYPE_NAME, &[ScalarType::Json]),
];

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
    ConnectorCapability::JsonFilteringArrayPath,
    ConnectorCapability::JsonFilteringAlphanumeric,
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

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        referential_integrity.allowed_referential_actions(NoAction | Restrict | Cascade | SetNull | SetDefault)
    }

    fn scalar_type_for_native_type(&self, native_type: serde_json::Value) -> ScalarType {
        let native_type: PostgresType = serde_json::from_value(native_type).unwrap();

        match native_type {
            //String
            Text => ScalarType::String,
            Char(_) => ScalarType::String,
            VarChar(_) => ScalarType::String,
            Bit(_) => ScalarType::String,
            VarBit(_) => ScalarType::String,
            Uuid => ScalarType::String,
            Xml => ScalarType::String,
            Inet => ScalarType::String,
            Citext => ScalarType::String,
            //Boolean
            Boolean => ScalarType::Boolean,
            //Int
            SmallInt => ScalarType::Int,
            Integer => ScalarType::Int,
            Oid => ScalarType::Int,
            //BigInt
            BigInt => ScalarType::BigInt,
            //Float
            Real => ScalarType::Float,
            DoublePrecision => ScalarType::Float,
            //Decimal
            Decimal(_) => ScalarType::Decimal,
            Money => ScalarType::Float,
            //DateTime
            Timestamp(_) => ScalarType::DateTime,
            Timestamptz(_) => ScalarType::DateTime,
            Date => ScalarType::DateTime,
            Time(_) => ScalarType::DateTime,
            Timetz(_) => ScalarType::DateTime,
            //Json
            Json => ScalarType::Json,
            JsonB => ScalarType::Json,
            //Bytes
            ByteA => ScalarType::Bytes,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {:?} in SCALAR_TYPE_DEFAULTS", scalar_type))
            .unwrap();

        serde_json::to_value(native_type).expect("PostgresType to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
        let native_type: PostgresType = serde_json::from_value(native_type).expect("PostgresType from JSON failed");

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && &native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: PostgresType =
            serde_json::from_value(native_type_instance.serialized_native_type.clone()).unwrap();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            Decimal(Some((prec, _))) if prec > 1000 || prec == 0 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    "Precision must be positive with a maximum value of 1000.",
                    span,
                ))
            }
            Bit(Some(0)) | VarBit(Some(0)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("M must be a positive integer.", span))
            }
            Timestamp(Some(p)) | Timestamptz(Some(p)) | Time(Some(p)) | Timetz(Some(p)) if p > 6 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 6.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, model: ModelWalker<'_>, errors: &mut Diagnostics) {
        for index in model.indexes() {
            validations::compatible_native_types(index, self, errors);
            validations::generalized_index_validations(index, self, errors);
            validations::spgist_indexed_column_count(index, errors);
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
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
        args: Vec<String>,
        span: Span,
    ) -> Result<NativeTypeInstance, DatamodelError> {
        let cloned_args = args.clone();

        let native_type = match name {
            SMALL_INT_TYPE_NAME => SmallInt,
            INTEGER_TYPE_NAME => Integer,
            BIG_INT_TYPE_NAME => BigInt,
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME, span)?),
            INET_TYPE_NAME => Inet,
            MONEY_TYPE_NAME => Money,
            CITEXT_TYPE_NAME => Citext,
            OID_TYPE_NAME => Oid,
            REAL_TYPE_NAME => Real,
            DOUBLE_PRECISION_TYPE_NAME => DoublePrecision,
            VARCHAR_TYPE_NAME => VarChar(parse_one_opt_u32(args, VARCHAR_TYPE_NAME, span)?),
            CHAR_TYPE_NAME => Char(parse_one_opt_u32(args, CHAR_TYPE_NAME, span)?),
            TEXT_TYPE_NAME => Text,
            BYTE_A_TYPE_NAME => ByteA,
            TIMESTAMP_TYPE_NAME => Timestamp(parse_one_opt_u32(args, TIMESTAMP_TYPE_NAME, span)?),
            TIMESTAMP_TZ_TYPE_NAME => Timestamptz(parse_one_opt_u32(args, TIMESTAMP_TZ_TYPE_NAME, span)?),
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time(parse_one_opt_u32(args, TIME_TYPE_NAME, span)?),
            TIME_TZ_TYPE_NAME => Timetz(parse_one_opt_u32(args, TIME_TZ_TYPE_NAME, span)?),
            BOOLEAN_TYPE_NAME => Boolean,
            BIT_TYPE_NAME => Bit(parse_one_opt_u32(args, BIT_TYPE_NAME, span)?),
            VAR_BIT_TYPE_NAME => VarBit(parse_one_opt_u32(args, VAR_BIT_TYPE_NAME, span)?),
            UUID_TYPE_NAME => Uuid,
            XML_TYPE_NAME => Xml,
            JSON_TYPE_NAME => Json,
            JSON_B_TYPE_NAME => JsonB,
            _ => return Err(DatamodelError::new_native_type_parser_error(name, span)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, native_type.to_json()))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> NativeTypeInstance {
        let native_type: PostgresType = serde_json::from_value(native_type).unwrap();
        let (constructor_name, args) = match native_type {
            SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            Integer => (INTEGER_TYPE_NAME, vec![]),
            BigInt => (BIG_INT_TYPE_NAME, vec![]),
            Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            Real => (REAL_TYPE_NAME, vec![]),
            DoublePrecision => (DOUBLE_PRECISION_TYPE_NAME, vec![]),
            VarChar(x) => (VARCHAR_TYPE_NAME, arg_vec_from_opt(x)),
            Char(x) => (CHAR_TYPE_NAME, arg_vec_from_opt(x)),
            Text => (TEXT_TYPE_NAME, vec![]),
            ByteA => (BYTE_A_TYPE_NAME, vec![]),
            Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            Timestamptz(x) => (TIMESTAMP_TZ_TYPE_NAME, arg_vec_from_opt(x)),
            Date => (DATE_TYPE_NAME, vec![]),
            Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            Timetz(x) => (TIME_TZ_TYPE_NAME, arg_vec_from_opt(x)),
            Boolean => (BOOLEAN_TYPE_NAME, vec![]),
            Bit(x) => (BIT_TYPE_NAME, arg_vec_from_opt(x)),
            VarBit(x) => (VAR_BIT_TYPE_NAME, arg_vec_from_opt(x)),
            Uuid => (UUID_TYPE_NAME, vec![]),
            Xml => (XML_TYPE_NAME, vec![]),
            Json => (JSON_TYPE_NAME, vec![]),
            JsonB => (JSON_B_TYPE_NAME, vec![]),
            Money => (MONEY_TYPE_NAME, vec![]),
            Inet => (INET_TYPE_NAME, vec![]),
            Citext => (CITEXT_TYPE_NAME, vec![]),
            Oid => (OID_TYPE_NAME, vec![]),
        };

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            NativeTypeInstance::new(constructor.name, args, native_type.to_json())
        } else {
            unreachable!()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err("must start with the protocol `postgresql://` or `postgres://`.".to_owned());
        }

        Ok(())
    }

    fn push_completions(
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
}

fn allowed_index_operator_classes(algo: IndexAlgorithm, field: ScalarFieldWalker<'_>) -> Vec<OperatorClass> {
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
