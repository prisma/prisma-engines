//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use crate::OutputFieldRef;
use indexmap::IndexMap;
use prisma_models::{OrderBy, PrismaValue, ScalarFieldRef};

pub type ParsedInputMap = IndexMap<String, ParsedInputValue>;

#[derive(Debug, Clone)]
pub struct ParsedObject {
    pub fields: Vec<FieldPair>,
}

#[derive(Debug, Clone)]
pub struct FieldPair {
    /// The field parsed from the incoming query.
    pub parsed_field: ParsedField,

    /// The schema field that the parsed field corresponds to.
    pub schema_field: OutputFieldRef,
}

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub name: String,
    pub alias: Option<String>,
    pub arguments: Vec<ParsedArgument>,
    pub nested_fields: Option<ParsedObject>,
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
