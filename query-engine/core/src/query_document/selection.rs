use crate::{ArgumentValue, ArgumentValueObject};
use indexmap::IndexMap;
use itertools::Itertools;
use schema::constants::filters;
use std::{borrow::Cow, collections::HashMap};

pub type SelectionArgument = (String, ArgumentValue);

#[derive(Debug, Clone)]
pub struct Selection {
    name: String,
    alias: Option<String>,
    arguments: Vec<(String, ArgumentValue)>,
    nested_selections: Vec<Selection>,
}

impl PartialEq for Selection {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.alias == other.alias
            && self.arguments.len() == other.arguments.len()
            && self.nested_selections.len() == other.nested_selections.len()
            && self.arguments.iter().all(|arg| other.arguments.contains(arg))
            && self
                .nested_selections
                .iter()
                .all(|sel| other.nested_selections.contains(sel))
    }
}

impl Selection {
    pub fn with_name(name: impl Into<String>) -> Selection {
        Selection::new(name.into(), None, Vec::new(), Vec::new())
    }

    pub fn new<T, A, N>(name: T, alias: Option<String>, arguments: A, nested_selections: N) -> Self
    where
        T: Into<String>,
        A: Into<Vec<SelectionArgument>>,
        N: Into<Vec<Selection>>,
    {
        Self {
            name: name.into(),
            alias,
            arguments: arguments.into(),
            nested_selections: nested_selections.into(),
        }
    }

    pub fn dedup(mut self) -> Self {
        self.nested_selections = self
            .nested_selections
            .into_iter()
            .unique_by(|s| s.name.clone())
            .collect();

        self
    }

    pub fn is_find_unique(&self) -> bool {
        self.name.starts_with("findUnique")
    }

    pub fn arguments(&self) -> &[(String, ArgumentValue)] {
        &self.arguments
    }

    pub fn pop_argument(&mut self) -> Option<(String, ArgumentValue)> {
        self.arguments.pop()
    }

    pub fn push_argument(&mut self, key: impl Into<String>, arg: impl Into<ArgumentValue>) {
        self.arguments.push((key.into(), arg.into()));
    }

    pub fn set_nested_selections(&mut self, sels: Vec<Selection>) {
        self.nested_selections = sels;
    }

    pub fn push_nested_selection(&mut self, selection: Selection) {
        self.nested_selections.push(selection);
    }

    pub fn contains_nested_selection(&self, name: &str) -> bool {
        self.nested_selections.iter().any(|sel| sel.name() == name)
    }

    pub fn nested_selections(&self) -> &[Self] {
        &self.nested_selections
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn alias(&self) -> &Option<String> {
        &self.alias
    }

    pub fn set_alias(&mut self, alias: Option<String>) {
        self.alias = alias
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionSet<'a> {
    // Single(Cow<'a, str>, Vec<ArgumentValue>),
    Single(Cow<'a, str>, Vec<(usize, ArgumentValue)>),
    // Multi(Vec<Vec<Cow<'a, str>>>, Vec<Vec<ArgumentValue>>),
    Multi(HashMap<usize, Vec<(Cow<'a, str>, ArgumentValue)>>),
    Empty,
}

impl<'a> Default for SelectionSet<'a> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<'a> SelectionSet<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(self, column: impl Into<Cow<'a, str>>, value: ArgumentValue, index: usize) -> Self {
        let column = column.into();

        match self {
            Self::Single(key, mut vals) if key == column => {
                vals.push((index, value));
                Self::Single(key, vals)
            }
            Self::Single(key, vals) => {
                let mut hm: HashMap<usize, Vec<(Cow<'_, str>, ArgumentValue)>> = HashMap::new();

                for val in vals {
                    hm.entry(val.0).or_default().push((key.clone(), val.1))
                }

                hm.entry(index).or_default().push((column, value));

                Self::Multi(hm)
            }
            Self::Multi(mut key_vals) => {
                key_vals.entry(index).or_default().push((column, value));

                Self::Multi(key_vals)
            }
            Self::Empty => Self::Single(column, vec![(index, value)]),
        }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_, _))
    }

    pub fn is_multi(&self) -> bool {
        matches!(self, Self::Multi(_))
    }

    pub fn keys(&self) -> Vec<&str> {
        match self {
            Self::Single(key, _) => vec![key.as_ref()],
            Self::Multi(keys) => keys
                .values()
                .flatten()
                .map(|(key, _)| key.as_ref())
                .unique()
                .collect_vec(),
            Self::Empty => Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct In<'a> {
    selection_set: SelectionSet<'a>,
}

impl<'a> In<'a> {
    pub fn new(selection_set: SelectionSet<'a>) -> Self {
        Self { selection_set }
    }
}

impl<'a> From<In<'a>> for ArgumentValue {
    fn from(other: In<'a>) -> Self {
        match other.selection_set {
            SelectionSet::Multi(key_vals) => {
                let conjuctive = key_vals.into_values().fold(Conjuctive::new(), |acc, key_val| {
                    let ands = key_val.into_iter().fold(Conjuctive::new(), |acc, (key, val)| {
                        let mut argument = IndexMap::new();
                        argument.insert(key.into_owned(), val);

                        acc.and(argument)
                    });

                    acc.or(ands)
                });

                ArgumentValue::from(conjuctive)
            }
            SelectionSet::Single(key, vals) => {
                let is_bool = vals.clone().into_iter().any(|(_, v)| match v {
                    ArgumentValue::Scalar(s) => matches!(s, query_structure::PrismaValue::Boolean(_)),
                    _ => false,
                });

                if is_bool {
                    let conjunctive = vals.into_iter().fold(Conjuctive::new(), |acc, (_, val)| {
                        let mut argument = IndexMap::new();

                        argument.insert(key.to_string(), val);
                        acc.or(argument)
                    });

                    return ArgumentValue::from(conjunctive);
                }

                ArgumentValue::object([(
                    key.to_string(),
                    ArgumentValue::object([(
                        filters::IN.to_owned(),
                        ArgumentValue::list(vals.into_iter().map(|(_, val)| val).collect_vec()),
                    )]),
                )])
            }
            SelectionSet::Empty => ArgumentValue::null(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Conjuctive {
    Or(Vec<Conjuctive>),
    And(Vec<Conjuctive>),
    Single(ArgumentValueObject),
    None,
}

impl From<ArgumentValueObject> for Conjuctive {
    fn from(map: ArgumentValueObject) -> Self {
        Self::Single(map)
    }
}

impl Default for Conjuctive {
    fn default() -> Self {
        Self::None
    }
}

impl Conjuctive {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn or(mut self, operation: impl Into<Conjuctive>) -> Self {
        match self {
            Self::Or(ref mut operations) => {
                operations.push(operation.into());
                self
            }
            Self::None => operation.into(),
            _ => Self::Or(vec![self, operation.into()]),
        }
    }

    pub fn and(mut self, operation: impl Into<Conjuctive>) -> Self {
        match self {
            Self::And(ref mut operations) => {
                operations.push(operation.into());
                self
            }
            Self::None => operation.into(),
            _ => Self::And(vec![self, operation.into()]),
        }
    }
}

impl From<Conjuctive> for ArgumentValue {
    fn from(conjuctive: Conjuctive) -> Self {
        match conjuctive {
            Conjuctive::None => Self::null(),
            Conjuctive::Single(obj) => ArgumentValue::object(single_to_multi_filter(obj)),
            Conjuctive::Or(conjuctives) => {
                let conditions: Vec<ArgumentValue> = conjuctives.into_iter().map(ArgumentValue::from).collect();

                ArgumentValue::object([("OR".to_string(), ArgumentValue::list(conditions))])
            }
            Conjuctive::And(conjuctives) => {
                let conditions: Vec<ArgumentValue> = conjuctives.into_iter().map(ArgumentValue::from).collect();

                ArgumentValue::object([("AND".to_string(), ArgumentValue::list(conditions))])
            }
        }
    }
}

/// Syntax for single-record and multi-record queries
fn single_to_multi_filter(obj: ArgumentValueObject) -> ArgumentValueObject {
    let mut new_obj: ArgumentValueObject = IndexMap::new();

    for (key, value) in obj {
        new_obj.insert(key, ArgumentValue::object([(filters::EQUALS.to_owned(), value)]));
    }

    new_obj
}
