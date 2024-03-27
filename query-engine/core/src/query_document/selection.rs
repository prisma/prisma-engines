use std::iter;

use crate::{ArgumentValue, ArgumentValueObject};
use indexmap::IndexMap;
use itertools::Itertools;
use schema::constants::filters;

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryFilters(Vec<(String, ArgumentValue)>);

impl QueryFilters {
    pub fn new(filters: Vec<(String, ArgumentValue)>) -> Self {
        Self(filters)
    }

    pub fn keys(&self) -> impl IntoIterator<Item = &str> + '_ {
        self.0.iter().map(|(key, _)| key.as_str())
    }

    pub fn has_many_keys(&self) -> bool {
        self.0.len() > 1
    }

    pub fn get_single_key(&self) -> Option<&(String, ArgumentValue)> {
        self.0.first()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionSet {
    Single(QuerySingle),
    Many(Vec<QueryFilters>),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuerySingle(String, Vec<ArgumentValue>);

impl QuerySingle {
    /// Attempt at building a single query filter from multiple query filters.
    /// Returns `None` if one of the query filters have more than one key.
    pub fn new(query_filters: &[QueryFilters]) -> Option<Self> {
        if query_filters.is_empty() {
            return None;
        }

        if query_filters.iter().any(|query_filters| query_filters.has_many_keys()) {
            return None;
        }

        let first = query_filters.first().unwrap();
        let (key, value) = first.get_single_key().unwrap();

        let mut result = QuerySingle(key.clone(), vec![value.clone()]);

        for filters in query_filters.iter().skip(1) {
            if let Some(single) = QuerySingle::push(result, filters) {
                result = single;
            } else {
                return None;
            }
        }

        Some(result)
    }

    fn push(mut previous: Self, next: &QueryFilters) -> Option<Self> {
        if next.0.is_empty() {
            Some(previous)
        // We have already validated that all `QueryFilters` have a single key.
        // So we can continue building it.
        } else {
            let (key, value) = next.0.first().unwrap();

            // if key matches, push value
            if key == &previous.0 {
                previous.1.push(value.clone());

                Some(previous)
            } else {
                // if key does not match, it's a many
                None
            }
        }
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::Empty
    }
}

impl SelectionSet {
    pub fn new(filters: Vec<QueryFilters>) -> Self {
        let single = QuerySingle::new(&filters);

        match single {
            Some(single) => SelectionSet::Single(single),
            None if filters.is_empty() => SelectionSet::Empty,
            None => SelectionSet::Many(filters),
        }
    }

    pub fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            Self::Single(single) => Box::new(iter::once(single.0.as_str())),
            Self::Many(filters) => Box::new(filters.iter().flat_map(|f| f.keys()).unique()),
            Self::Empty => Box::new(iter::empty()),
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
                let conjuctive = buckets.into_iter().fold(Conjuctive::new(), |acc, bucket| {
                    // Needed because we flush the last bucket by pushing an empty one, which gets translated to a `Null` as the Conjunctive is empty.
                    let ands = bucket.0.into_iter().fold(Conjuctive::new(), |acc, (key, value)| {
                        let mut argument = IndexMap::with_capacity(1);
                        argument.insert(key.clone(), value);

                        acc.and(argument)
                    });

                    acc.or(ands)
                });

                ArgumentValue::from(conjuctive)
            }
            SelectionSet::Single(QuerySingle(key, vals)) => {
                let is_bool = vals.clone().into_iter().any(|v| match v {
                    ArgumentValue::Scalar(s) => matches!(s, query_structure::PrismaValue::Boolean(_)),
                    _ => false,
                });

                if is_bool {
                    let conjunctive = vals.into_iter().fold(Conjuctive::new(), |acc, val| {
                        let mut argument = IndexMap::new();

                        argument.insert(key.to_string(), val);
                        acc.or(argument)
                    });

                    return ArgumentValue::from(conjunctive);
                }

                ArgumentValue::object([(
                    key.to_string(),
                    ArgumentValue::object([(filters::IN.to_owned(), ArgumentValue::list(vals))]),
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
