//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use std::ops::{Deref, DerefMut};

use crate::{ObjectTag, OutputFieldRef};
use indexmap::IndexMap;
use prisma_models::{OrderBy, PrismaValue, ScalarFieldRef};

pub type ParsedInputList = Vec<ParsedInputValue>;

#[derive(Debug, Clone, Default)]
pub struct ParsedInputMap {
    pub tag: Option<ObjectTag>,
    pub map: IndexMap<String, ParsedInputValue>,
}

impl ParsedInputMap {
    pub fn set_tag(&mut self, tag: Option<ObjectTag>) {
        self.tag = tag;
    }
}

impl From<IndexMap<String, ParsedInputValue>> for ParsedInputMap {
    fn from(map: IndexMap<String, ParsedInputValue>) -> Self {
        Self { tag: None, map }
    }
}

impl FromIterator<(String, ParsedInputValue)> for ParsedInputMap {
    fn from_iter<T: IntoIterator<Item = (String, ParsedInputValue)>>(iter: T) -> Self {
        Self {
            tag: None,
            map: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for ParsedInputMap {
    type Item = (String, ParsedInputValue);
    type IntoIter = indexmap::map::IntoIter<String, ParsedInputValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl Deref for ParsedInputMap {
    type Target = IndexMap<String, ParsedInputValue>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for ParsedInputMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

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
    List(ParsedInputList),
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
