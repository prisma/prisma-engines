#[derive(Debug, Clone, PartialEq)]
pub enum IntType {
    Signed8,
    Signed16,
    Signed24,
    Signed32,
    Unsigned8,
    Unsigned16,
    Unsigned24,
    Unsigned32,
    Custom(i64, i64),
}

impl std::fmt::Display for IntType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntType::Signed8 => f.write_str("8-bit signed integer"),
            IntType::Signed16 => f.write_str("16-bit signed integer"),
            IntType::Signed24 => f.write_str("24-bit signed integer"),
            IntType::Signed32 => f.write_str("32-bit signed integer"),
            IntType::Unsigned8 => f.write_str("8-bit unsigned integer"),
            IntType::Unsigned16 => f.write_str("16-bit unsigned integer"),
            IntType::Unsigned24 => f.write_str("24-bit unsigned integer"),
            IntType::Unsigned32 => f.write_str("32-bit unsigned integer"),
            IntType::Custom(min, max) => write!(f, "custom integer (min: {}, max: {})", min, max),
        }
    }
}
