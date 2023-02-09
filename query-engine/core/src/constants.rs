pub mod custom_types {
    use prisma_models::PrismaValue;

    pub const TYPE: &str = "$type";
    pub const VALUE: &str = "value";

    pub const DATETIME: &str = "DateTime";
    pub const BIGINT: &str = "BigInt";
    pub const DECIMAL: &str = "Decimal";
    pub const BYTES: &str = "Bytes";
    pub const JSON: &str = "Json";
    pub const ENUM: &str = "Enum";
    pub const FIELD_REF: &str = "FieldRef";

    pub fn make_object(typ: &str, value: PrismaValue) -> PrismaValue {
        PrismaValue::Object(vec![make_type_pair(typ), make_value_pair(value)])
    }

    fn make_type_pair(typ: &str) -> (String, PrismaValue) {
        (TYPE.to_string(), PrismaValue::String(typ.to_string()))
    }

    fn make_value_pair(value: PrismaValue) -> (String, PrismaValue) {
        (VALUE.to_string(), value)
    }
}
