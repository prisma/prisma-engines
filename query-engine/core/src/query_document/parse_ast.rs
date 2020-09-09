//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use crate::OutputFieldRef;
use indexmap::IndexMap;
use prisma_models::{OrderBy, PrismaValue, ScalarFieldRef};

pub type ParsedInputMap = IndexMap<String, ParsedInputValue>;

#[derive(Debug, Clone)]
pub struct ParsedObject {
    pub fields: Vec<ParsedField>,
}

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub name: String,
    pub alias: Option<String>,
    pub arguments: Vec<ParsedArgument>,
    pub nested_fields: Option<ParsedObject>,

    /// Associated schema field
    pub schema_field: OutputFieldRef,
}

/// Indicator for a query that should be ran as-is in the database, as plain
/// SQL.
#[derive(Debug, Clone, Copy)]
pub enum RawQueryType {
    /// Execute the query and return the number of changed rows.
    Execute,
    /// Execute the query, returning rows from the database.
    Query,
}

impl ParsedField {
    /// For raw SQL queries, returns the expected type of the result sets.
    pub fn raw_query_type(&self) -> Option<RawQueryType> {
        match self.name.as_str() {
            "executeRaw" => Some(RawQueryType::Execute),
            "queryRaw" => Some(RawQueryType::Query),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedArgument {
    pub name: String,
    pub value: ParsedInputValue,
}

#[derive(Debug, Clone)]
pub enum ParsedInputValue {
    Single(PrismaValue),
    OrderBy(OrderBy),
    ScalarField(ScalarFieldRef),
    List(Vec<ParsedInputValue>),
    Map(ParsedInputMap),
}

impl ParsedArgument {
    pub fn into_value(self) -> Option<PrismaValue> {
        match self.value {
            ParsedInputValue::Single(val) => Some(val),
            _ => None,
        }
    }
}

pub trait ArgumentListLookup {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument>;
}

impl ArgumentListLookup for Vec<ParsedArgument> {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument> {
        self.iter().position(|arg| arg.name == name).map(|pos| self.remove(pos))
    }
}
