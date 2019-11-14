use super::*;

/// Postgres Example Impl
struct SimpleConnector {
    aliases: Vec<TypeAlias>,
    field_type_constructors: Vec<FieldTypeConstructor>,
}

impl Connector for SimpleConnector {
    fn type_aliases(&self) -> &Vec<TypeAlias> {
        &self.aliases
    }

    fn field_type_constructors(&self) -> &Vec<FieldTypeConstructor> {
        &self.field_type_constructors
    }
}

impl SimpleConnector {
    pub fn postgres() -> SimpleConnector {
        let aliases = vec![
            // standard types
            TypeAlias::new("String", "Text"),
            //            TypeAlias::new("Boolean", "Boolean"),
            TypeAlias::new("Int", "Integer"),
            TypeAlias::new("String", "Text"),
            TypeAlias::new("String", "Text"),
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
        /// missing because of interpolation:
        /// Numeric, Character, CharacterVarying, Timestamp, TimestampWithTimeZone, Time
        /// Bit, BitVarying
        ///
        /// types for which photon types are unclear:
        /// ByteA, Date, TimeTZ
        /// Point, Line, LSeg, Box, Path, Polygon, Circle
        /// CIDR, INet, Macaddr
        /// TSVector, TSQuery
        /// UUID
        /// XML, JSON, JSONB
        /// Int4Range, Int8Range, NumRange, TSRange, TSTZRange, DateRange
        /// TXIDSnapshot
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
        SimpleConnector {
            aliases,
            field_type_constructors,
        }
    }
}
