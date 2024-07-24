use super::ColumnType;

pub struct ParsedRawQuery {
    pub parameters: Vec<ParsedRawItem>,
    pub columns: Vec<ParsedRawItem>,
}

pub struct ParsedRawItem {
    pub name: String,
    pub typ: ColumnType,
}
