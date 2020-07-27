use super::{declarative_connector::*, Connector, ScalarType};
use crate::ConnectorCapability;

pub struct BuiltinConnectors {}

impl BuiltinConnectors {
    // returns a connector representing the intersection of all provided connectors
    pub fn combined(connectors: Vec<Box<dyn Connector>>) -> Box<dyn Connector> {
        // the standard library does not seem to offer an elegant way to do this. Don't want to pull in a dependency for this.
        let mut combined_capabilities = vec![];
        for connector in &connectors {
            for capability in connector.capabilities() {
                let supported_by_all_connectors = connectors.iter().all(|c| c.has_capability(*capability));

                if supported_by_all_connectors {
                    combined_capabilities.push(*capability);
                }
            }
        }

        Box::new(DeclarativeConnector {
            type_aliases: Vec::new(),
            field_type_constructors: Vec::new(),
            capabilities: combined_capabilities,
        })
    }

    pub fn sqlite() -> DeclarativeConnector {
        DeclarativeConnector {
            type_aliases: vec![],
            field_type_constructors: vec![],
            capabilities: vec![],
        }
    }

    pub fn mssql() -> DeclarativeConnector {
        DeclarativeConnector {
            type_aliases: vec![],
            field_type_constructors: vec![],
            capabilities: vec![],
        }
    }

    pub fn mysql() -> DeclarativeConnector {
        DeclarativeConnector {
            type_aliases: vec![],
            field_type_constructors: vec![],
            capabilities: vec![
                ConnectorCapability::RelationsOverNonUniqueCriteria,
                ConnectorCapability::Enums,
                ConnectorCapability::Json,
                ConnectorCapability::MultipleIndexesWithSameName,
            ],
        }
    }

    pub fn postgres() -> DeclarativeConnector {
        let type_aliases = vec![
            // standard types
            TypeAlias::new("String", "Text"),
            //            TypeAlias::new("Boolean", "Boolean"),
            TypeAlias::new("Int", "Integer"),
            TypeAlias::new("Float", "Real"),
            TypeAlias::new("DateTime", "Timestamp"),
            // custom types
            TypeAlias::new("Int8", "BigInt"),
            TypeAlias::new("Serial8", "BigSerial"),
            TypeAlias::new("Float8", "DoublePrecision"),
            TypeAlias::new("Int", "Integer"),
            TypeAlias::new("Int4", "Integer"),
            TypeAlias::new("Decimal", "Numeric"),
            TypeAlias::new("Float4", "Real"),
            TypeAlias::new("Int2", "SmallInt"),
            TypeAlias::new("Serial2", "SmallSerial"),
            TypeAlias::new("Serial4", "Serial"),
            TypeAlias::new("Char", "Character"),
            TypeAlias::new("VarChar", "CharacterVarying"),
            TypeAlias::new("TimestampTZ", "TimestampWithTimeZone"),
            TypeAlias::new("Bool", "Boolean"),
            TypeAlias::new("VarBit", "BitVarying"),
        ];
        // missing because of interpolation:
        // Numeric, Character, CharacterVarying, Timestamp, TimestampWithTimeZone, Time
        // Bit, BitVarying
        //
        // types for which photon types are unclear:
        // ByteA, Date, TimeTZ
        // Point, Line, LSeg, Box, Path, Polygon, Circle
        // CIDR, INet, Macaddr
        // TSVector, TSQuery
        // UUID
        // XML, JSON, JSONB
        // Int4Range, Int8Range, NumRange, TSRange, TSTZRange, DateRange
        // TXIDSnapshot
        let field_type_constructors = vec![
            FieldTypeConstructor::without_args("BigInt", "bigint", ScalarType::Int),
            FieldTypeConstructor::without_args("BigSerial", "bigserial", ScalarType::Int),
            FieldTypeConstructor::without_args("DoublePrecision", "double precision", ScalarType::Float),
            FieldTypeConstructor::without_args("Integer", "integer", ScalarType::Int),
            FieldTypeConstructor::without_args("Real", "real", ScalarType::Float),
            FieldTypeConstructor::without_args("SmallInt", "smallint", ScalarType::Int),
            FieldTypeConstructor::without_args("SmallSerial", "smallserial", ScalarType::Int),
            FieldTypeConstructor::without_args("Serial", "serial", ScalarType::Int),
            FieldTypeConstructor::without_args("Money", "money", ScalarType::Float),
            FieldTypeConstructor::without_args("Text", "text", ScalarType::String),
            FieldTypeConstructor::without_args("Char", "char", ScalarType::String),
            FieldTypeConstructor::without_args("Name", "name", ScalarType::String),
            FieldTypeConstructor::without_args("Boolean", "boolean", ScalarType::Boolean),
            FieldTypeConstructor::without_args("Boolean", "boolean", ScalarType::Boolean),
            FieldTypeConstructor::without_args("PGLSN", "pg_lsn", ScalarType::Int),
        ];
        DeclarativeConnector {
            type_aliases,
            field_type_constructors,
            capabilities: vec![
                ConnectorCapability::ScalarLists,
                ConnectorCapability::Enums,
                ConnectorCapability::Json,
            ],
        }
    }
}
