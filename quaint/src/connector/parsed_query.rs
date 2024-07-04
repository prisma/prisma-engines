pub trait IntoParsedQueryType {
    fn as_parsed_query_type(&self) -> ParsedQueryType;
}

pub struct ParsedRawQuery {
    pub parameters: Vec<ParsedRawItem>,
    pub columns: Vec<ParsedRawItem>,
}

pub struct ParsedRawItem {
    pub name: String,
    pub typ: ParsedQueryType,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ParsedQueryType {
    Null,
    Int32,
    Int64,
    Float,
    Double,
    Text,
    Enum,
    Bytes,
    Boolean,
    Char,
    Numeric,
    Json,
    Xml,
    Uuid,
    DateTime,
    Date,
    Time,

    Int32Array,
    Int64Array,
    FloatArray,
    DoubleArray,
    TextArray,
    BytesArray,
    BooleanArray,
    CharArray,
    NumericArray,
    JsonArray,
    XmlArray,
    UuidArray,
    DateTimeArray,
    DateArray,
    TimeArray,

    UnknownArray,
    Unknown,
}

impl std::fmt::Display for ParsedQueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsedQueryType::Null => write!(f, "null"),
            ParsedQueryType::Int32 => write!(f, "int32"),
            ParsedQueryType::Int64 => write!(f, "int64"),
            ParsedQueryType::Float => write!(f, "float"),
            ParsedQueryType::Double => write!(f, "double"),
            ParsedQueryType::Text => write!(f, "text"),
            ParsedQueryType::Enum => write!(f, "enum"),
            ParsedQueryType::Bytes => write!(f, "bytes"),
            ParsedQueryType::Boolean => write!(f, "boolean"),
            ParsedQueryType::Char => write!(f, "char"),
            ParsedQueryType::Numeric => write!(f, "numeric"),
            ParsedQueryType::Json => write!(f, "json"),
            ParsedQueryType::Xml => write!(f, "xml"),
            ParsedQueryType::Uuid => write!(f, "uuid"),
            ParsedQueryType::DateTime => write!(f, "dateTime"),
            ParsedQueryType::Date => write!(f, "date"),
            ParsedQueryType::Time => write!(f, "time"),
            ParsedQueryType::Int32Array => write!(f, "int32Array"),
            ParsedQueryType::Int64Array => write!(f, "int64Array"),
            ParsedQueryType::FloatArray => write!(f, "floatArray"),
            ParsedQueryType::DoubleArray => write!(f, "doubleArray"),
            ParsedQueryType::TextArray => write!(f, "textArray"),
            ParsedQueryType::BytesArray => write!(f, "bytesArray"),
            ParsedQueryType::BooleanArray => write!(f, "booleanArray"),
            ParsedQueryType::CharArray => write!(f, "charArray"),
            ParsedQueryType::NumericArray => write!(f, "numericArray"),
            ParsedQueryType::JsonArray => write!(f, "jsonArray"),
            ParsedQueryType::XmlArray => write!(f, "xmlArray"),
            ParsedQueryType::UuidArray => write!(f, "uuidArray"),
            ParsedQueryType::DateTimeArray => write!(f, "dateTimeArray"),
            ParsedQueryType::DateArray => write!(f, "dateArray"),
            ParsedQueryType::TimeArray => write!(f, "timeArray"),
            ParsedQueryType::UnknownArray => write!(f, "unknownArray"),
            ParsedQueryType::Unknown => write!(f, "unknown"),
        }
    }
}
