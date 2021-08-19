use datamodel_connector::{
    connector_error::ConnectorError,
    helper::{arg_vec_from_opt, args_vec_from_opt, parse_one_opt_u32, parse_two_opt_u32},
    Connector, ConnectorCapability,
};
use dml::{
    field::{Field, FieldType},
    model::Model,
    native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance,
    relation_info::ReferentialAction,
    scalars::ScalarType,
};
use enumflags2::BitFlags;
use native_types::PostgresType::{self, *};

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

pub struct PostgresDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
    referential_actions: BitFlags<ReferentialAction>,
}

//todo should this also contain the pretty printed output for SQL rendering?
impl PostgresDatamodelConnector {
    pub fn new() -> PostgresDatamodelConnector {
        use ReferentialAction::*;

        let capabilities = vec![
            ConnectorCapability::ScalarLists,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
            ConnectorCapability::AutoIncrementMultipleAllowed,
            ConnectorCapability::AutoIncrementAllowedOnNonId,
            ConnectorCapability::AutoIncrementNonIndexedAllowed,
            ConnectorCapability::InsensitiveFilters,
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::CreateMany,
            ConnectorCapability::WritableAutoincField,
            ConnectorCapability::CreateSkipDuplicates,
            ConnectorCapability::UpdateableId,
            ConnectorCapability::JsonFilteringArrayPath,
            ConnectorCapability::CreateManyWriteableAutoIncId,
            ConnectorCapability::AutoIncrement,
            ConnectorCapability::CompoundIds,
            ConnectorCapability::ForeignKeys,
            ConnectorCapability::AnyId,
            ConnectorCapability::QueryRaw,
            ConnectorCapability::NamedPrimaryKeys,
            ConnectorCapability::NamedForeignKeys,
            ConnectorCapability::ForeignKeys,
            ConnectorCapability::FullTextSearchWithoutIndex,
        ];

        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, vec![ScalarType::Int]);
        let integer = NativeTypeConstructor::without_args(INTEGER_TYPE_NAME, vec![ScalarType::Int]);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, vec![ScalarType::BigInt]);
        let decimal = NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let money = NativeTypeConstructor::without_args(MONEY_TYPE_NAME, vec![ScalarType::Decimal]);
        let inet = NativeTypeConstructor::without_args(INET_TYPE_NAME, vec![ScalarType::String]);
        let citext = NativeTypeConstructor::without_args(CITEXT_TYPE_NAME, vec![ScalarType::String]);
        let oid = NativeTypeConstructor::without_args(OID_TYPE_NAME, vec![ScalarType::Int]);
        let real = NativeTypeConstructor::without_args(REAL_TYPE_NAME, vec![ScalarType::Float]);
        let double_precision = NativeTypeConstructor::without_args(DOUBLE_PRECISION_TYPE_NAME, vec![ScalarType::Float]);
        let varchar = NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let char = NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, vec![ScalarType::String]);
        let byte_a = NativeTypeConstructor::without_args(BYTE_A_TYPE_NAME, vec![ScalarType::Bytes]);
        let timestamp = NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let timestamptz =
            NativeTypeConstructor::with_optional_args(TIMESTAMP_TZ_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, vec![ScalarType::DateTime]);
        let time = NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let timetz = NativeTypeConstructor::with_optional_args(TIME_TZ_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let boolean = NativeTypeConstructor::without_args(BOOLEAN_TYPE_NAME, vec![ScalarType::Boolean]);
        let bit = NativeTypeConstructor::with_optional_args(BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let varbit = NativeTypeConstructor::with_optional_args(VAR_BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let uuid = NativeTypeConstructor::without_args(UUID_TYPE_NAME, vec![ScalarType::String]);
        let xml = NativeTypeConstructor::without_args(XML_TYPE_NAME, vec![ScalarType::String]);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, vec![ScalarType::Json]);
        let json_b = NativeTypeConstructor::without_args(JSON_B_TYPE_NAME, vec![ScalarType::Json]);

        let constructors = vec![
            small_int,
            integer,
            big_int,
            decimal,
            money,
            inet,
            citext,
            oid,
            real,
            double_precision,
            varchar,
            char,
            text,
            byte_a,
            timestamp,
            timestamptz,
            date,
            time,
            timetz,
            boolean,
            bit,
            varbit,
            uuid,
            xml,
            json,
            json_b,
        ];

        let referential_actions = NoAction | Restrict | Cascade | SetNull | SetDefault;

        PostgresDatamodelConnector {
            capabilities,
            constructors,
            referential_actions,
        }
    }
}

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
    fn name(&self) -> &str {
        "Postgres"
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn constraint_name_length(&self) -> usize {
        63
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        self.referential_actions
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

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        match field.field_type() {
            FieldType::Scalar(_scalar_type, _, Some(native_type_instance)) => {
                let native_type: PostgresType = native_type_instance.deserialize_native_type();
                let error = self.native_instance_error(native_type_instance);

                match native_type {
                    Decimal(Some((precision, scale))) if scale > precision => {
                        error.new_scale_larger_than_precision_error()
                    }
                    Decimal(Some((prec, _))) if prec > 1000 || prec == 0 => error
                        .new_argument_m_out_of_range_error("Precision must be positive with a maximum value of 1000."),
                    Bit(Some(0)) | VarBit(Some(0)) => {
                        error.new_argument_m_out_of_range_error("M must be a positive integer.")
                    }
                    Timestamp(Some(p)) | Timestamptz(Some(p)) | Time(Some(p)) | Timetz(Some(p)) if p > 6 => {
                        error.new_argument_m_out_of_range_error("M can range from 0 to 6.")
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn validate_model(&self, model: &Model) -> Result<(), ConnectorError> {
        for index_definition in model.indices.iter() {
            let fields = index_definition.fields.iter().map(|f| model.find_field(f).unwrap());

            for field in fields {
                if let FieldType::Scalar(_, _, Some(native_type)) = field.field_type() {
                    let r#type: PostgresType = native_type.deserialize_native_type();
                    let error = self.native_instance_error(native_type);

                    if r#type == PostgresType::Xml {
                        return if index_definition.is_unique() {
                            error.new_incompatible_native_type_with_unique()
                        } else {
                            error.new_incompatible_native_type_with_index()
                        };
                    }
                }
            }
        }

        Ok(())
    }

    fn available_native_type_constructors(&self) -> &[NativeTypeConstructor] {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let cloned_args = args.clone();

        let native_type = match name {
            SMALL_INT_TYPE_NAME => SmallInt,
            INTEGER_TYPE_NAME => Integer,
            BIG_INT_TYPE_NAME => BigInt,
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME)?),
            INET_TYPE_NAME => Inet,
            MONEY_TYPE_NAME => Money,
            CITEXT_TYPE_NAME => Citext,
            OID_TYPE_NAME => Oid,
            REAL_TYPE_NAME => Real,
            DOUBLE_PRECISION_TYPE_NAME => DoublePrecision,
            VARCHAR_TYPE_NAME => VarChar(parse_one_opt_u32(args, VARCHAR_TYPE_NAME)?),
            CHAR_TYPE_NAME => Char(parse_one_opt_u32(args, CHAR_TYPE_NAME)?),
            TEXT_TYPE_NAME => Text,
            BYTE_A_TYPE_NAME => ByteA,
            TIMESTAMP_TYPE_NAME => Timestamp(parse_one_opt_u32(args, TIMESTAMP_TYPE_NAME)?),
            TIMESTAMP_TZ_TYPE_NAME => Timestamptz(parse_one_opt_u32(args, TIMESTAMP_TZ_TYPE_NAME)?),
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time(parse_one_opt_u32(args, TIME_TYPE_NAME)?),
            TIME_TZ_TYPE_NAME => Timetz(parse_one_opt_u32(args, TIME_TZ_TYPE_NAME)?),
            BOOLEAN_TYPE_NAME => Boolean,
            BIT_TYPE_NAME => Bit(parse_one_opt_u32(args, BIT_TYPE_NAME)?),
            VAR_BIT_TYPE_NAME => VarBit(parse_one_opt_u32(args, VAR_BIT_TYPE_NAME)?),
            UUID_TYPE_NAME => Uuid,
            XML_TYPE_NAME => Xml,
            JSON_TYPE_NAME => Json,
            JSON_B_TYPE_NAME => JsonB,
            _ => return Err(ConnectorError::new_native_type_parser_error(name)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, &native_type))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
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
            Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type))
        } else {
            self.native_str_error(constructor_name).native_type_name_unknown()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err("must start with the protocol `postgresql://` or `postgres://`.".to_owned());
        }

        Ok(())
    }
}

impl Default for PostgresDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}
