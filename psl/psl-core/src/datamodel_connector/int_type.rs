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
            IntType::Signed8 => write!(f, "8-bit signed integer"),
            IntType::Signed16 => write!(f, "16-bit signed integer"),
            IntType::Signed24 => write!(f, "24-bit signed integer"),
            IntType::Signed32 => write!(f, "32-bit signed integer"),
            IntType::Unsigned8 => write!(f, "8-bit unsigned integer"),
            IntType::Unsigned16 => write!(f, "16-bit unsigned integer"),
            IntType::Unsigned24 => write!(f, "24-bit unsigned integer"),
            IntType::Unsigned32 => write!(f, "32-bit unsigned integer"),
            IntType::Custom(min, max) => write!(f, "custom integer (min: {}, max: {})", min, max),
        }
    }
}
