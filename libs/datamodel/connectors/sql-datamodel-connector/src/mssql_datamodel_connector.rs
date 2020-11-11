use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::{
    field::{Field, FieldType},
    model::{IndexType, Model},
    native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance,
};
use native_types::{
    MsSqlType::{self, *},
    TypeParameter::*,
};
use std::{borrow::Cow, convert::TryFrom};

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
            .map(|kind| NativeTypeConstructor::from(*kind))
            .collect();

        MsSqlDatamodelConnector {
            capabilities,
            constructors,
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
                // We've validated the type earlier (hopefully).
                let r#type = MsSqlType::try_from(native_type).unwrap();

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
                    Text | NText | Image | Xml => {
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
                    VarChar(Some(Max)) | NVarChar(Some(Max)) | VarBinary(Some(Max)) => {
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

            let incompatible_index_type = |kind: &str| {
                if index_definition.tpe == IndexType::Unique {
                    Err(ConnectorError::new_incompatible_native_type_with_unique(
                        kind,
                        "SQL Server",
                    ))
                } else {
                    Err(ConnectorError::new_incompatible_native_type_with_index(
                        kind,
                        "SQL Server",
                    ))
                }
            };

            for field in fields {
                if let FieldType::NativeType(_, native_type) = field.field_type() {
                    // We've validated the type earlier (hopefully).
                    let r#type = MsSqlType::try_from(native_type).unwrap();

                    match r#type {
                        Text | NText | Image | Xml => incompatible_index_type(&format!("{}", r#type))?,
                        VarChar(Some(Max)) | NVarChar(Some(Max)) | VarBinary(Some(Max)) => {
                            incompatible_index_type(&format!("{}", r#type))?
                        }
                        _ => {}
                    }
                }
            }
        }

        for id_field in model.id_fields.iter() {
            let field = model.find_field(id_field).unwrap();

            if let FieldType::NativeType(_, native_type) = field.field_type() {
                // We've validated the type earlier (hopefully).
                let r#type = MsSqlType::try_from(native_type).unwrap();

                match r#type {
                    Text | NText | Image | Xml => {
                        return Err(ConnectorError::new_incompatible_native_type_with_id(
                            &format!("{}", r#type),
                            "SQL Server",
                        ));
                    }
                    VarChar(Some(Max)) | NVarChar(Some(Max)) | VarBinary(Some(Max)) => {
                        return Err(ConnectorError::new_incompatible_native_type_with_id(
                            &format!("{}", r#type),
                            "SQL Server",
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let qualified = if args.len() > 0 {
            Cow::from(format!("{}({})", name, args.join(",")))
        } else {
            Cow::from(name)
        };

        // Unwrapping as the core must guarantee to just call with known names.
        let native_type: MsSqlType = qualified.parse().unwrap();

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
