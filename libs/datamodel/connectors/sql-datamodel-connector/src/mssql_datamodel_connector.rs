use std::borrow::Cow;

use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::field::FieldType;
use dml::model::IndexType;
use dml::{
    field::Field, model::Model, native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance,
};
use native_types::MsSqlType;
use native_types::{MsSqlKind, TypeParameter};

static ENABLED_NATIVE_TYPES: &[MsSqlKind] = &[
    MsSqlKind::TinyInt,
    MsSqlKind::SmallInt,
    MsSqlKind::Int,
    MsSqlKind::BigInt,
    MsSqlKind::Decimal,
    MsSqlKind::Numeric,
    MsSqlKind::Money,
    MsSqlKind::SmallMoney,
    MsSqlKind::Bit,
    MsSqlKind::Float,
    MsSqlKind::Real,
    MsSqlKind::Date,
    MsSqlKind::Time,
    MsSqlKind::DateTime,
    MsSqlKind::DateTime2,
    MsSqlKind::DateTimeOffset,
    MsSqlKind::SmallDateTime,
    MsSqlKind::Char,
    MsSqlKind::NChar,
    MsSqlKind::VarChar,
    MsSqlKind::Text,
    MsSqlKind::NVarChar,
    MsSqlKind::NText,
    MsSqlKind::Binary,
    MsSqlKind::VarBinary,
    MsSqlKind::Image,
    MsSqlKind::Xml,
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
                // We've validated the kind already earlier (hopefully).
                let kind: MsSqlKind = native_type.name.parse().unwrap();
                let args = native_type.args.as_slice();

                match kind {
                    MsSqlKind::Decimal | MsSqlKind::Numeric => match args {
                        [precision, scale] if scale > precision => Err(
                            ConnectorError::new_scale_larger_than_precision_error(kind.as_ref(), "SQL Server"),
                        ),
                        [precision, _] if *precision > TypeParameter::Number(38) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Precision can range from 1 to 38.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        [precision, _] if *precision == TypeParameter::Number(0) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Precision can range from 1 to 38.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        [_, scale] if *scale > TypeParameter::Number(38) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Scale can range from 0 to 38.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        _ => Ok(()),
                    },
                    MsSqlKind::Float => match args {
                        [bits] if *bits == TypeParameter::Number(0) || *bits > TypeParameter::Number(53) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Bits can range from 1 to 53.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        _ => Ok(()),
                    },
                    MsSqlKind::Text | MsSqlKind::NText | MsSqlKind::Image | MsSqlKind::Xml => {
                        if field.is_unique() {
                            Err(ConnectorError::new_incompatible_native_type_with_unique(
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        } else if field.is_id() {
                            Err(ConnectorError::new_incompatible_native_type_with_id(
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    MsSqlKind::VarChar | MsSqlKind::NVarChar | MsSqlKind::VarBinary => match args {
                        [length] if *length == TypeParameter::Max => {
                            if field.is_unique() {
                                let typ = format!("{}({})", kind, TypeParameter::Max);

                                Err(ConnectorError::new_incompatible_native_type_with_unique(
                                    &typ,
                                    "SQL Server",
                                ))
                            } else if field.is_id() {
                                let typ = format!("{}({})", kind, TypeParameter::Max);
                                Err(ConnectorError::new_incompatible_native_type_with_id(&typ, "SQL Server"))
                            } else {
                                Ok(())
                            }
                        }
                        [length] if *length > TypeParameter::Number(2000) && kind == MsSqlKind::NVarChar => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Length can range from 1 to 2000. For larger sizes, use the `Max` variant.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        [length] if *length > TypeParameter::Number(4000) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Length can range from 1 to 4000. For larger sizes, use the `Max` variant.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        _ => Ok(()),
                    },
                    MsSqlKind::NChar => match args {
                        [length] if *length > TypeParameter::Number(2000) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Length can range from 1 to 2000.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        _ => Ok(()),
                    },
                    MsSqlKind::Char | MsSqlKind::Binary => match args {
                        [length] if *length > TypeParameter::Number(4000) => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Length can range from 1 to 4000.",
                                kind.as_ref(),
                                "SQL Server",
                            ))
                        }
                        _ => Ok(()),
                    },
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
                    let kind: MsSqlKind = native_type.name.parse().unwrap();
                    let args = native_type.args.as_slice();

                    match kind {
                        MsSqlKind::Text | MsSqlKind::NText | MsSqlKind::Image | MsSqlKind::Xml => {
                            incompatible_index_type(kind.as_ref())?
                        }
                        MsSqlKind::VarChar | MsSqlKind::NVarChar | MsSqlKind::VarBinary => match args {
                            [length] if *length == TypeParameter::Max => {
                                let typ = format!("{}({})", kind, TypeParameter::Max);
                                incompatible_index_type(&typ)?
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }

        for id_field in model.id_fields.iter() {
            let field = model.find_field(id_field).unwrap();

            if let FieldType::NativeType(_, native_type) = field.field_type() {
                let kind: MsSqlKind = native_type.name.parse().unwrap();
                let args = native_type.args.as_slice();

                match kind {
                    MsSqlKind::Text | MsSqlKind::NText | MsSqlKind::Image | MsSqlKind::Xml => {
                        return Err(ConnectorError::new_incompatible_native_type_with_id(
                            kind.as_ref(),
                            "SQL Server",
                        ));
                    }
                    MsSqlKind::VarChar | MsSqlKind::NVarChar | MsSqlKind::VarBinary => match args {
                        [length] if *length == TypeParameter::Max => {
                            let typ = format!("{}({})", kind, TypeParameter::Max);

                            return Err(ConnectorError::new_incompatible_native_type_with_id(&typ, "SQL Server"));
                        }
                        _ => {}
                    },
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
        let (kind, parameters) = native_type.clone().into_parts();

        Ok(NativeTypeInstance::new(kind.as_ref(), parameters, &native_type))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MsSqlType = serde_json::from_value(native_type).unwrap();
        let (kind, parameters) = native_type.clone().into_parts();

        if let Some(_) = self.find_native_type_constructor(kind.as_ref()) {
            Ok(NativeTypeInstance::new(kind.as_ref(), parameters, &native_type))
        } else {
            Err(ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: kind.as_ref().to_string(),
                connector_name: "SQL Server".to_string(),
            }))
        }
    }
}
