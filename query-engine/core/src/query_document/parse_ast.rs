//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use super::{QueryParserError, QueryParserResult};
use crate::FieldRef;
use prisma_models::{OrderBy, PrismaValue};
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

/// Note: Assertions should live on the schema level and run through the validation as any other check.
///       This requires a slightly larger refactoring.
pub trait InputAssertions: Sized {
    /// Asserts the exact size of the underlying input.
    fn assert_size(&self, size: usize) -> QueryParserResult<()>;

    /// Asserts that the given value is non-null. The exact definition of what is considered
    /// null or non-null is dependent on the implementor.
    fn assert_non_null(&self) -> QueryParserResult<()>;
}

impl InputAssertions for ParsedInputValue {
    fn assert_size(&self, size: usize) -> QueryParserResult<()> {
        match self {
            Self::List(v) => v.assert_size(size)?,
            Self::Map(m) => m.assert_size(size)?,
            _ => unimplemented!(),
        };

        Ok(())
    }

    fn assert_non_null(&self) -> QueryParserResult<()> {
        match self {
            Self::List(v) => v.assert_non_null()?,
            Self::Map(m) => m.assert_non_null()?,
            Self::Single(v) => v.assert_non_null()?,
            Self::OrderBy(_) => (),
        };

        Ok(())
    }
}

impl InputAssertions for ParsedInputMap {
    fn assert_size(&self, size: usize) -> QueryParserResult<()> {
        if self.len() != size {
            Err(QueryParserError::AssertionError(format!(
                "Expected object to have exactly {} key-value pairs, got: {} {}",
                size,
                self.len(),
                Some(self.iter().map(|v| v.0.as_str()).collect::<Vec<&str>>().join(", "))
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("({})", s))
                    .unwrap_or_else(String::new)
            )))
        } else {
            Ok(())
        }
    }

    fn assert_non_null(&self) -> QueryParserResult<()> {
        for (_, value) in self.iter() {
            value.assert_non_null()?;
        }

        Ok(())
    }
}

impl InputAssertions for Vec<ParsedInputValue> {
    fn assert_size(&self, size: usize) -> QueryParserResult<()> {
        if self.len() != size {
            Err(QueryParserError::AssertionError(format!(
                "Expected list to have exactly {} input values, got: {}.",
                size,
                self.len()
            )))
        } else {
            Ok(())
        }
    }

    /// Asserts that all elements are non-null
    fn assert_non_null(&self) -> QueryParserResult<()> {
        for input in self.iter() {
            input.assert_non_null()?;
        }

        Ok(())
    }
}

impl InputAssertions for PrismaValue {
    fn assert_size(&self, _size: usize) -> QueryParserResult<()> {
        unimplemented!()
    }

    fn assert_non_null(&self) -> QueryParserResult<()> {
        match self {
            PrismaValue::Null => Err(QueryParserError::AssertionError(format!(
                "You provided a null value for a where clause (or implicit nested selector). Please provide a non null value.",
            ))),
            _ => Ok(())
        }
    }
}
