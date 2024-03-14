//! Parsed query document tree. Naming is WIP.
//! Structures represent parsed and validated parts of the query document, used by the query builders.
use crate::QueryParserResult;
use indexmap::IndexMap;
use query_structure::{OrderBy, PrismaValue, ScalarFieldRef};
use schema::ObjectTag;
use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

pub(crate) type ParsedInputList<'a> = Vec<ParsedInputValue<'a>>;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedInputMap<'a> {
    pub(crate) tag: Option<ObjectTag<'a>>,
    pub(crate) map: IndexMap<Cow<'a, str>, ParsedInputValue<'a>>,
}

impl<'a> ParsedInputMap<'a> {
    pub(crate) fn set_tag(&mut self, tag: Option<ObjectTag<'a>>) {
        self.tag = tag;
    }

    pub(crate) fn is_relation_envelope(&self) -> bool {
        matches!(&self.tag, Some(ObjectTag::RelationEnvelope))
    }

    pub(crate) fn is_composite_envelope(&self) -> bool {
        matches!(&self.tag, Some(ObjectTag::CompositeEnvelope))
    }

    pub(crate) fn is_nested_to_one_update_envelope(&self) -> bool {
        matches!(&self.tag, Some(ObjectTag::NestedToOneUpdateEnvelope))
    }
}

impl<'a> From<IndexMap<Cow<'a, str>, ParsedInputValue<'a>>> for ParsedInputMap<'a> {
    fn from(map: IndexMap<Cow<'a, str>, ParsedInputValue<'a>>) -> Self {
        Self { tag: None, map }
    }
}

impl<'a> FromIterator<(Cow<'a, str>, ParsedInputValue<'a>)> for ParsedInputMap<'a> {
    fn from_iter<T: IntoIterator<Item = (Cow<'a, str>, ParsedInputValue<'a>)>>(iter: T) -> Self {
        Self {
            tag: None,
            map: iter.into_iter().collect(),
        }
    }
}

impl<'a> IntoIterator for ParsedInputMap<'a> {
    type Item = (Cow<'a, str>, ParsedInputValue<'a>);
    type IntoIter = indexmap::map::IntoIter<Cow<'a, str>, ParsedInputValue<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> Deref for ParsedInputMap<'a> {
    type Target = IndexMap<Cow<'a, str>, ParsedInputValue<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<'a> DerefMut for ParsedInputMap<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[derive(Debug, Clone)]
pub struct ParsedObject<'a> {
    pub(crate) fields: Vec<FieldPair<'a>>,
}

#[derive(Debug, Clone)]
pub struct FieldPair<'a> {
    /// The field parsed from the incoming query.
    pub(crate) parsed_field: ParsedField<'a>,

    /// The schema field that the parsed field corresponds to.
    pub(crate) schema_field: schema::OutputField<'a>,
}

#[derive(Clone, Debug)]
pub struct ParsedField<'a> {
    pub(crate) name: String,
    pub(crate) alias: Option<String>,
    pub(crate) arguments: Vec<ParsedArgument<'a>>,
    pub(crate) nested_fields: Option<ParsedObject<'a>>,
}

impl<'a> ParsedField<'a> {
    pub(crate) fn where_arg(&mut self) -> QueryParserResult<Option<ParsedInputMap<'a>>> {
        self.look_arg("where")
    }

    pub(crate) fn create_arg(&mut self) -> QueryParserResult<Option<ParsedInputMap<'a>>> {
        self.look_arg("create")
    }

    pub(crate) fn update_arg(&mut self) -> QueryParserResult<Option<ParsedInputMap<'a>>> {
        self.look_arg("update")
    }

    pub(crate) fn has_nested_selection(&self) -> bool {
        self.nested_fields
            .as_ref()
            .map(|nested_field| {
                nested_field
                    .fields
                    .iter()
                    .any(|field| field.parsed_field.nested_fields.is_some())
            })
            .unwrap_or(false)
    }

    fn look_arg(&mut self, arg_name: &str) -> QueryParserResult<Option<ParsedInputMap<'a>>> {
        self.arguments
            .lookup(arg_name)
            .as_ref()
            .map(|arg| arg.value.clone().try_into())
            .transpose()
    }
}

#[derive(Debug, Clone)]
pub struct ParsedArgument<'a> {
    pub(crate) name: String,
    pub(crate) value: ParsedInputValue<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedInputValue<'a> {
    Single(PrismaValue),
    OrderBy(OrderBy),
    ScalarField(ScalarFieldRef),
    List(ParsedInputList<'a>),
    Map(ParsedInputMap<'a>),
}

pub(crate) trait ArgumentListLookup<'a> {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument<'a>>;
}

impl<'a> ArgumentListLookup<'a> for Vec<ParsedArgument<'a>> {
    fn lookup(&mut self, name: &str) -> Option<ParsedArgument<'a>> {
        self.iter().position(|arg| arg.name == name).map(|pos| self.remove(pos))
    }
}
