use query_structure::PrismaValue;

pub fn coerce_null_to_zero_value(value: PrismaValue) -> PrismaValue {
    if let PrismaValue::Null = value {
        PrismaValue::Int(0)
    } else {
        value
    }
}
