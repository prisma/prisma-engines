use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::scalars::ScalarType;
use dml::{
    field::{Field, FieldType},
    model::{IndexType, Model},
    native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance,
};
use native_types::{
    MsSqlType::{self, *},
    MsSqlTypeParameter::*,
};
use std::borrow::Cow;

static ENABLED_NATIVE_TYPES: &[MsSqlType] = &[
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Decimal(None),
    Numeric(None),
    Money,
    SmallMoney,
    Bit,
    Float(None),
    Real,
    Date,
    Time,
    DateTime,
    DateTime2,
    DateTimeOffset,
    SmallDateTime,
    Char(None),
    NChar(None),
    VarChar(None),
    Text,
    NVarChar(None),
    NText,
    Binary(None),
    VarBinary(None),
    Image,
    Xml,
    UniqueIdentifier,
];

pub struct MsSqlDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl MsSqlDatamodelConnector {
    pub fn new() -> MsSqlDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::AutoIncrementAllowedOnNonId,
            ConnectorCapability::AutoIncrementMultipleAllowed,
            ConnectorCapability::AutoIncrementNonIndexedAllowed,
        ];

        let constructors: Vec<_> = ENABLED_NATIVE_TYPES
            .into_iter()
            .map(|kind| Self::constructor_for(*kind))
            .collect();

        MsSqlDatamodelConnector {
            capabilities,
            constructors,
        }
    }

    pub fn constructor_for(r#type: MsSqlType) -> NativeTypeConstructor {
        let matching_types = match r#type {
            MsSqlType::TinyInt => vec![ScalarType::Int],
            MsSqlType::SmallInt => vec![ScalarType::Int],
            MsSqlType::Int => vec![ScalarType::Int],
            MsSqlType::BigInt => vec![ScalarType::BigInt],
            MsSqlType::Decimal(_) => vec![ScalarType::Decimal],
            MsSqlType::Numeric(_) => vec![ScalarType::Decimal],
            MsSqlType::Money => vec![ScalarType::Float],
            MsSqlType::SmallMoney => vec![ScalarType::Float],
            MsSqlType::Bit => vec![ScalarType::Boolean, ScalarType::Int],
            MsSqlType::Float(_) => vec![ScalarType::Float],
            MsSqlType::Real => vec![ScalarType::Float],
            MsSqlType::Date => vec![ScalarType::DateTime],
            MsSqlType::Time => vec![ScalarType::DateTime],
            MsSqlType::DateTime => vec![ScalarType::DateTime],
            MsSqlType::DateTime2 => vec![ScalarType::DateTime],
            MsSqlType::DateTimeOffset => vec![ScalarType::DateTime],
            MsSqlType::SmallDateTime => vec![ScalarType::DateTime],
            MsSqlType::Char(_) => vec![ScalarType::String],
            MsSqlType::NChar(_) => vec![ScalarType::String],
            MsSqlType::VarChar(_) => vec![ScalarType::String],
            MsSqlType::Text => vec![ScalarType::String],
            MsSqlType::NVarChar(_) => vec![ScalarType::String],
            MsSqlType::NText => vec![ScalarType::String],
            MsSqlType::Binary(_) => vec![ScalarType::Bytes],
            MsSqlType::VarBinary(_) => vec![ScalarType::Bytes],
            MsSqlType::Image => vec![ScalarType::Bytes],
            MsSqlType::Xml => vec![ScalarType::String],
            MsSqlType::UniqueIdentifier => vec![ScalarType::String],
        };

        match r#type.maximum_parameters() {
            0 => NativeTypeConstructor::without_args(r#type.kind(), matching_types),
            n => NativeTypeConstructor::with_optional_args(r#type.kind(), n, matching_types),
        }
    }
}

impl Connector for MsSqlDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        match field.field_type() {
            FieldType::NativeType(_, native_type) => {
                let r#type: MsSqlType = native_type.deserialize_native_type();

                match r#type {
                    Decimal(Some(params)) | Numeric(Some(params)) => match params {
                        (precision, scale) if scale > precision => Err(
                            ConnectorError::new_scale_larger_than_precision_error(&format!("{}", r#type), "SQL Server"),
                        ),
                        (precision, _) if precision == 0 || precision > 38 => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Precision can range from 1 to 38.",
                                &format!("{}", r#type),
                                "SQL Server",
                            ))
                        }
                        (_, scale) if scale > 38 => Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Scale can range from 0 to 38.",
                            &format!("{}", r#type),
                            "SQL Server",
                        )),
                        _ => Ok(()),
                    },
                    Float(Some(bits)) => match bits {
                        bits if bits == 0 || bits > 53 => Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Bits can range from 1 to 53.",
                            &format!("{}", r#type),
                            "SQL Server",
                        )),
                        _ => Ok(()),
                    },
                    typ if MsSqlType::heap_allocated().contains(&typ) => {
                        if field.is_unique() {
                            Err(ConnectorError::new_incompatible_native_type_with_unique(
                                &format!("{}", r#type),
                                "SQL Server",
                            ))
                        } else if field.is_id() {
                            Err(ConnectorError::new_incompatible_native_type_with_id(
                                &format!("{}", r#type),
                                "SQL Server",
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    NVarChar(Some(Number(p))) if p > 2000 => Err(ConnectorError::new_argument_m_out_of_range_error(
                        "Length can range from 1 to 2000. For larger sizes, use the `Max` variant.",
                        &format!("{}", r#type),
                        "SQL Server",
                    )),
                    VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if p > 4000 => {
                        Err(ConnectorError::new_argument_m_out_of_range_error(
                            r#"Length can range from 1 to 4000. For larger sizes, use the `Max` variant."#,
                            &format!("{}", r#type),
                            "SQL Server",
                        ))
                    }
                    NChar(Some(p)) if p > 2000 => Err(ConnectorError::new_argument_m_out_of_range_error(
                        "Length can range from 1 to 2000.",
                        &format!("{}", r#type),
                        "SQL Server",
                    )),
                    Char(Some(p)) | Binary(Some(p)) if p > 4000 => {
                        Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Length can range from 1 to 4000.",
                            &format!("{}", r#type),
                            "SQL Server",
                        ))
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
                if let FieldType::NativeType(_, native_type) = field.field_type() {
                    let r#type: MsSqlType = native_type.deserialize_native_type();

                    if MsSqlType::heap_allocated().contains(&r#type) {
                        if index_definition.tpe == IndexType::Unique {
                            return Err(ConnectorError::new_incompatible_native_type_with_unique(
                                &format!("{}", r#type),
                                "SQL Server",
                            ));
                        } else {
                            return Err(ConnectorError::new_incompatible_native_type_with_index(
                                &format!("{}", r#type),
                                "SQL Server",
                            ));
                        }
                    }
                }
            }
        }

        for id_field in model.id_fields.iter() {
            let field = model.find_field(id_field).unwrap();

            if let FieldType::NativeType(_, native_type) = field.field_type() {
                let r#type: MsSqlType = native_type.deserialize_native_type();

                if MsSqlType::heap_allocated().contains(&r#type) {
                    return Err(ConnectorError::new_incompatible_native_type_with_id(
                        &format!("{}", r#type),
                        "SQL Server",
                    ));
                }
            }
        }

        Ok(())
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: &[String]) -> Result<NativeTypeInstance, ConnectorError> {
        let qualified = if args.len() > 0 {
            Cow::from(format!("{}({})", name, args.join(",")))
        } else {
            Cow::from(name)
        };

        let native_type: MsSqlType = qualified.parse()?;

        Ok(NativeTypeInstance::new(
            native_type.kind(),
            native_type.parameters(),
            &native_type,
        ))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MsSqlType = serde_json::from_value(native_type).unwrap();
        let kind = native_type.kind();
        let parameters = native_type.parameters();

        if let Some(_) = self.find_native_type_constructor(kind) {
            Ok(NativeTypeInstance::new(kind, parameters, &native_type))
        } else {
            Err(ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: kind.to_string(),
                connector_name: "SQL Server".to_string(),
            }))
        }
    }
}
