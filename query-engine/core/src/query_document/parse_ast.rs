//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use super::{QueryParserError, QueryParserResult};
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

/// Note: Assertions should live on the schema level and run through the validation as any other check.
///       This requires a slightly larger refactoring.
pub trait InputAssertions: Sized {
    /// Asserts the exact size of the underlying input.
    fn assert_size(self, size: usize) -> QueryParserResult<Self>;
}

impl InputAssertions for ParsedInputMap {
    fn assert_size(self, size: usize) -> QueryParserResult<Self> {
        if self.len() != size {
            Err(QueryParserError::AssertionError(format!(
                "Expected object to have exactly {} key-value pairs, got: {} ({})",
                size,
                self.len(),
                self.iter().map(|v| v.0.as_str()).collect::<Vec<&str>>().join(", ")
            )))
        } else {
            Ok(self)
        }
    }
}
