mod validations;

use connection_string::JdbcString;
use datamodel::datamodel_connector::{
    helper::{arg_vec_from_opt, args_vec_from_opt, parse_one_opt_u32, parse_two_opt_u32},
    parser_database::{self, ast, ParserDatabase, ScalarType},
    Connector, ConnectorCapability, ConstraintScope, DatamodelError, Diagnostics, NativeTypeConstructor,
    NativeTypeInstance, ReferentialAction, ReferentialIntegrity, Span,
};
use enumflags2::BitFlags;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};
use native_types::{MsSqlType, MsSqlTypeParameter, NativeType};
use once_cell::sync::Lazy;
use std::borrow::Cow;

use MsSqlType::*;
use MsSqlTypeParameter::*;

const TINY_INT_TYPE_NAME: &str = "TinyInt";
const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const INT_TYPE_NAME: &str = "Int";
const BIG_INT_TYPE_NAME: &str = "BigInt";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const NUMERIC_TYPE_NAME: &str = "Numeric";
const MONEY_TYPE_NAME: &str = "Money";
const SMALL_MONEY_TYPE_NAME: &str = "SmallMoney";
const BIT_TYPE_NAME: &str = "Bit";
const FLOAT_TYPE_NAME: &str = "Float";
const REAL_TYPE_NAME: &str = "Real";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const DATETIME_TYPE_NAME: &str = "DateTime";
const DATETIME2_TYPE_NAME: &str = "DateTime2";
const DATETIME_OFFSET_TYPE_NAME: &str = "DateTimeOffset";
const SMALL_DATETIME_TYPE_NAME: &str = "SmallDateTime";
const CHAR_TYPE_NAME: &str = "Char";
const NCHAR_TYPE_NAME: &str = "NChar";
const VARCHAR_TYPE_NAME: &str = "VarChar";
const TEXT_TYPE_NAME: &str = "Text";
const NVARCHAR_TYPE_NAME: &str = "NVarChar";
const NTEXT_TYPE_NAME: &str = "NText";
const BINARY_TYPE_NAME: &str = "Binary";
const VAR_BINARY_TYPE_NAME: &str = "VarBinary";
const IMAGE_TYPE_NAME: &str = "Image";
const XML_TYPE_NAME: &str = "Xml";
const UNIQUE_IDENTIFIER_TYPE_NAME: &str = "UniqueIdentifier";

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[
    NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, &[ScalarType::BigInt]),
    NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, &[ScalarType::Decimal]),
    NativeTypeConstructor::with_optional_args(NUMERIC_TYPE_NAME, 2, &[ScalarType::Decimal]),
    NativeTypeConstructor::without_args(MONEY_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(SMALL_MONEY_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(BIT_TYPE_NAME, &[ScalarType::Boolean, ScalarType::Int]),
    NativeTypeConstructor::with_optional_args(FLOAT_TYPE_NAME, 1, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(REAL_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(DATE_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(TIME_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(DATETIME_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(DATETIME2_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(DATETIME_OFFSET_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(SMALL_DATETIME_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(NCHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::without_args(TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(NVARCHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::without_args(NTEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(BINARY_TYPE_NAME, 1, &[ScalarType::Bytes]),
    NativeTypeConstructor::with_optional_args(VAR_BINARY_TYPE_NAME, 1, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(IMAGE_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(XML_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(UNIQUE_IDENTIFIER_TYPE_NAME, &[ScalarType::String]),
];

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[
    ConstraintScope::GlobalPrimaryKeyForeignKeyDefault,
    ConstraintScope::ModelPrimaryKeyKeyIndex,
];

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::AnyId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::AutoIncrementAllowedOnNonId,
    ConnectorCapability::AutoIncrementMultipleAllowed,
    ConnectorCapability::AutoIncrementNonIndexedAllowed,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::CreateMany,
    ConnectorCapability::MultiSchema,
    ConnectorCapability::NamedDefaultValues,
    ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::NamedPrimaryKeys,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::ReferenceCycleDetection,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::PrimaryKeySortOrderDefinition,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::ClusteringSetting,
    ConnectorCapability::OrderByNullsFirstLast,
    ConnectorCapability::SupportsTxIsolationReadUncommitted,
    ConnectorCapability::SupportsTxIsolationReadCommitted,
    ConnectorCapability::SupportsTxIsolationRepeatableRead,
    ConnectorCapability::SupportsTxIsolationSerializable,
    ConnectorCapability::SupportsTxIsolationSnapshot,
];

pub struct MsSqlDatamodelConnector;

impl MsSqlDatamodelConnector {
    fn parse_mssql_type_parameter(
        &self,
        r#type: &str,
        args: &[String],
        span: Span,
    ) -> Result<Option<MsSqlTypeParameter>, DatamodelError> {
        static MAX_REGEX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^(?i)max$").unwrap());
        static NUM_REGEX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^\d+$").unwrap());

        match args {
            [] => Ok(None),
            [s] if MAX_REGEX.is_match(s) => Ok(Some(MsSqlTypeParameter::Max)),
            [s] if NUM_REGEX.is_match(s) => Ok(s.trim().parse().map(MsSqlTypeParameter::Number).ok()),
            s => Err(DatamodelError::new_invalid_native_type_argument(
                r#type,
                &s.join(","),
                "a number or `Max`",
                span,
            )),
        }
    }
}

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

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        128
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        referential_integrity.allowed_referential_actions(NoAction | Cascade | SetNull | SetDefault)
    }

    fn scalar_type_for_native_type(&self, native_type: serde_json::Value) -> ScalarType {
        let native_type: MsSqlType = serde_json::from_value(native_type).unwrap();

        match native_type {
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
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {:?} in SCALAR_TYPE_DEFAULTS", scalar_type))
            .unwrap();

        serde_json::to_value(native_type).expect("MsSqlType to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
        let native_type: MsSqlType = serde_json::from_value(native_type).expect("MsSqlType from JSON failed");

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && &native_type == nt)
    }

    fn set_config_dir<'a>(&self, config_dir: &std::path::Path, url: &'a str) -> Cow<'a, str> {
        let mut jdbc: JdbcString = match format!("jdbc:{url}").parse() {
            Ok(jdbc) => jdbc,
            _ => return Cow::from(url),
        };

        let set_root = |path: String| {
            let path = std::path::Path::new(&path);

            if path.is_relative() {
                Some(config_dir.join(&path).to_str().map(ToString::to_string).unwrap())
            } else {
                Some(path.to_str().unwrap().to_string())
            }
        };

        let props = jdbc.properties_mut();

        let cert_path = props.remove("trustservercertificateca").and_then(set_root);

        if let Some(path) = cert_path {
            props.insert("trustServerCertificateCA".to_owned(), path);
        }

        let final_connection_string = format!("{jdbc}").replace("jdbc:sqlserver://", "sqlserver://");

        Cow::Owned(final_connection_string)
    }

    fn validate_native_type_arguments(
        &self,
        native_type: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let r#type: MsSqlType = serde_json::from_value(native_type.serialized_native_type.clone()).unwrap();
        let error = self.native_instance_error(native_type);

        match r#type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span));
            }
            Decimal(Some((prec, _))) if prec == 0 || prec > 38 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 38.", span));
            }
            Decimal(Some((_, scale))) if scale > 38 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 38.", span))
            }
            Float(Some(bits)) if bits == 0 || bits > 53 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Bits can range from 1 to 53.", span))
            }
            NVarChar(Some(Number(p))) if p > 4000 => errors.push_error(error.new_argument_m_out_of_range_error(
                "Length can range from 1 to 4000. For larger sizes, use the `Max` variant.",
                span,
            )),
            VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if p > 8000 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    r#"Length can range from 1 to 8000. For larger sizes, use the `Max` variant."#,
                    span,
                ))
            }
            NChar(Some(p)) if p > 4000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 4000.", span))
            }
            Char(Some(p)) | Binary(Some(p)) if p > 8000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 8000.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, model: parser_database::walkers::ModelWalker<'_>, errors: &mut Diagnostics) {
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
        NATIVE_TYPE_CONSTRUCTORS
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: Vec<String>,
        span: Span,
    ) -> Result<NativeTypeInstance, DatamodelError> {
        let cloned_args = args.clone();
        let native_type = match name {
            TINY_INT_TYPE_NAME => TinyInt,
            SMALL_INT_TYPE_NAME => SmallInt,
            INT_TYPE_NAME => Int,
            BIG_INT_TYPE_NAME => BigInt,
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME, span)?),
            MONEY_TYPE_NAME => Money,
            SMALL_MONEY_TYPE_NAME => SmallMoney,
            BIT_TYPE_NAME => Bit,
            FLOAT_TYPE_NAME => Float(parse_one_opt_u32(args, FLOAT_TYPE_NAME, span)?),
            REAL_TYPE_NAME => Real,
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time,
            DATETIME_TYPE_NAME => DateTime,
            DATETIME2_TYPE_NAME => DateTime2,
            DATETIME_OFFSET_TYPE_NAME => DateTimeOffset,
            SMALL_DATETIME_TYPE_NAME => SmallDateTime,
            CHAR_TYPE_NAME => Char(parse_one_opt_u32(args, CHAR_TYPE_NAME, span)?),
            NCHAR_TYPE_NAME => NChar(parse_one_opt_u32(args, NCHAR_TYPE_NAME, span)?),
            VARCHAR_TYPE_NAME => VarChar(self.parse_mssql_type_parameter(name, &args, span)?),
            TEXT_TYPE_NAME => Text,
            NVARCHAR_TYPE_NAME => NVarChar(self.parse_mssql_type_parameter(name, &args, span)?),
            NTEXT_TYPE_NAME => NText,
            BINARY_TYPE_NAME => Binary(parse_one_opt_u32(args, BINARY_TYPE_NAME, span)?),
            VAR_BINARY_TYPE_NAME => VarBinary(self.parse_mssql_type_parameter(name, &args, span)?),
            IMAGE_TYPE_NAME => Image,
            XML_TYPE_NAME => Xml,
            UNIQUE_IDENTIFIER_TYPE_NAME => UniqueIdentifier,
            _ => return Err(DatamodelError::new_native_type_parser_error(name, span)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, native_type.to_json()))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> NativeTypeInstance {
        let native_type: MsSqlType = serde_json::from_value(native_type).unwrap();

        let (constructor_name, args) = match native_type {
            TinyInt => (TINY_INT_TYPE_NAME, vec![]),
            SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            Int => (INT_TYPE_NAME, vec![]),
            BigInt => (BIG_INT_TYPE_NAME, vec![]),
            Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            Money => (MONEY_TYPE_NAME, vec![]),
            SmallMoney => (SMALL_MONEY_TYPE_NAME, vec![]),
            Bit => (BIT_TYPE_NAME, vec![]),
            Float(x) => (FLOAT_TYPE_NAME, arg_vec_from_opt(x)),
            Real => (REAL_TYPE_NAME, vec![]),
            Date => (DATE_TYPE_NAME, vec![]),
            Time => (TIME_TYPE_NAME, vec![]),
            DateTime => (DATETIME_TYPE_NAME, vec![]),
            DateTime2 => (DATETIME2_TYPE_NAME, vec![]),
            DateTimeOffset => (DATETIME_OFFSET_TYPE_NAME, vec![]),
            SmallDateTime => (SMALL_DATETIME_TYPE_NAME, vec![]),
            Char(x) => (CHAR_TYPE_NAME, arg_vec_from_opt(x)),
            NChar(x) => (NCHAR_TYPE_NAME, arg_vec_from_opt(x)),
            VarChar(x) => (VARCHAR_TYPE_NAME, arg_vec_for_type_param(x)),
            Text => (TEXT_TYPE_NAME, vec![]),
            NVarChar(x) => (NVARCHAR_TYPE_NAME, arg_vec_for_type_param(x)),
            NText => (NTEXT_TYPE_NAME, vec![]),
            Binary(x) => (BINARY_TYPE_NAME, arg_vec_from_opt(x)),
            VarBinary(x) => (VAR_BINARY_TYPE_NAME, arg_vec_for_type_param(x)),
            Image => (IMAGE_TYPE_NAME, vec![]),
            Xml => (XML_TYPE_NAME, vec![]),
            UniqueIdentifier => (UNIQUE_IDENTIFIER_TYPE_NAME, vec![]),
        };

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            let stringified_args = args.iter().map(|arg| arg.to_string()).collect();
            NativeTypeInstance::new(constructor.name, stringified_args, native_type.to_json())
        } else {
            unreachable!()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("sqlserver") {
            return Err("must start with the protocol `sqlserver://`.".to_string());
        }

        Ok(())
    }

    fn push_completions(
        &self,
        _db: &ParserDatabase,
        position: ast::SchemaPosition<'_>,
        completions: &mut CompletionList,
    ) {
        if let ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("default", _, None)),
        ) = position
        {
            completions.items.push(CompletionItem {
                label: "map: ".to_owned(),
                kind: Some(CompletionItemKind::PROPERTY),
                ..Default::default()
            });
        }
    }
}

/// A collection of types stored outside of the row to the heap, having
/// certain properties such as not allowed in keys or normal indices.
pub fn heap_allocated_types() -> &'static [MsSqlType] {
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

fn arg_vec_for_type_param(type_param: Option<MsSqlTypeParameter>) -> Vec<String> {
    match type_param {
        Some(MsSqlTypeParameter::Max) => vec!["Max".to_string()],
        Some(MsSqlTypeParameter::Number(l)) => vec![l.to_string()],
        None => vec![],
    }
}
