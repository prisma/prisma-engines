use crate::{ArgumentValue, ArgumentValueObject};
use indexmap::IndexMap;
use itertools::Itertools;
use schema::constants::filters;
use std::collections::HashMap;

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
pub struct QueryFilters(Vec<(String, ArgumentValue)>);

impl QueryFilters {
    pub fn new(filters: Vec<(String, ArgumentValue)>) -> Self {
        Self(filters)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionSet {
    Single(String, Vec<ArgumentValue>),
    Many(Vec<HashMap<String, Vec<ArgumentValue>>>),
    Empty,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::Empty
    }
}

impl SelectionSet {
    pub fn new(filters: Vec<QueryFilters>) -> Self {
        filters.into_iter().fold(Self::default(), |acc, query_filters| {
            acc.push(query_filters).flush_many()
        })
    }

    fn push(self, query_filters: QueryFilters) -> Self {
        query_filters
            .0
            .into_iter()
            .fold(self, |acc, (new_key, new_value)| match acc {
                SelectionSet::Single(key, mut values) if key == new_key => {
                    values.push(new_value);

                    SelectionSet::Single(key, values)
                }
                SelectionSet::Single(key, values) => {
                    let mut map: HashMap<_, _> = HashMap::from_iter([(key, values)]);

                    map.entry(new_key).or_default().push(new_value);

                    SelectionSet::Many(vec![map])
                }
                SelectionSet::Many(mut maps) => {
                    maps.last_mut().unwrap().entry(new_key).or_default().push(new_value);

                    SelectionSet::Many(maps)
                }
                SelectionSet::Empty => SelectionSet::Single(new_key, vec![new_value]),
            })
    }

    fn flush_many(self) -> Self {
        match self {
            SelectionSet::Many(mut maps) => {
                maps.push(HashMap::new());

                SelectionSet::Many(maps)
            }
            _ => self,
        }
    }

    pub fn keys(&self) -> Vec<&str> {
        match self {
            Self::Single(key, _) => vec![key.as_ref()],
            Self::Many(filters) => filters
                .iter()
                .flat_map(|f| f.keys())
                .map(|key| key.as_ref())
                .unique()
                .collect(),
            Self::Empty => Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct In {
    selection_set: SelectionSet,
}

impl In {
    pub fn new(selection_set: SelectionSet) -> Self {
        Self { selection_set }
    }
}

impl From<In> for ArgumentValue {
    fn from(other: In) -> Self {
        match other.selection_set {
            SelectionSet::Many(buckets) => {
                let conjuctive = buckets.into_iter().fold(Conjuctive::new(), |acc, filters| {
                    // Needed because we flush the last bucket by pushing an empty one, which gets translated to a `Null` as the Conjunctive is empty.
                    if !filters.is_empty() {
                        let ands = filters.into_iter().fold(Conjuctive::new(), |mut acc, (key, values)| {
                            for value in values {
                                let mut argument = IndexMap::with_capacity(1);
                                argument.insert(key.clone(), value);

                                acc = acc.and(argument)
                            }

                            acc
                        });

                        acc.or(ands)
                    } else {
                        acc
                    }
                });

                dbg!(&conjuctive);

                ArgumentValue::from(conjuctive)
            }
            SelectionSet::Single(key, vals) => ArgumentValue::object([(
                key.to_string(),
                ArgumentValue::object([(filters::IN.to_owned(), ArgumentValue::list(vals))]),
            )]),
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
