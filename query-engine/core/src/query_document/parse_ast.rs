//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use crate::FieldRef;
use prisma_models::PrismaValue;
use std::collections::BTreeMap;

pub type ParsedInputMap = BTreeMap<String, ParsedInputValue>;

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
    pub schema_field: FieldRef,
}

#[derive(Debug, Clone)]
pub struct ParsedArgument {
    pub name: String,
    pub value: ParsedInputValue,
}

#[derive(Debug, Clone)]
pub enum ParsedInputValue {
    Single(PrismaValue),
    List(Vec<ParsedInputValue>),
    Map(ParsedInputMap),
}

pub trait ArgumentListLookup {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument>;
}

impl ArgumentListLookup for Vec<ParsedArgument> {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument> {
        self.iter().position(|arg| arg.name == name).map(|pos| self.remove(pos))
    }
}
