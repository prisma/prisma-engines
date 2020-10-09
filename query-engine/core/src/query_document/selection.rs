use super::QueryValue;
use indexmap::IndexMap;
use itertools::Itertools;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionBuilder {
    name: String,
    alias: Option<String>,
    arguments: Vec<(String, QueryValue)>,
    nested_selections: Vec<Selection>,
}

impl SelectionBuilder {
    pub fn alias(&mut self, value: impl Into<String>) -> &mut Self {
        self.alias = Some(value.into());
        self
    }

    pub fn arguments(&self) -> &[(String, QueryValue)] {
        &self.arguments
    }

    pub fn set_arguments(&mut self, value: Vec<(String, QueryValue)>) -> &mut Self {
        self.arguments = value;
        self
    }

    pub fn push_argument(&mut self, key: impl Into<String>, arg: impl Into<QueryValue>) -> &mut Self {
        self.arguments.push((key.into(), arg.into()));
        self
    }

    pub fn nested_selections(&mut self, sels: Vec<Selection>) -> &mut Self {
        self.nested_selections = sels;
        self
    }

    pub fn push_nested_selection(&mut self, selection: Selection) -> &mut Self {
        self.nested_selections.push(selection);
        self
    }

    pub fn contains_nested_selection(&self, name: &str) -> bool {
        self.nested_selections.iter().any(|sel| sel.name() == name)
    }

    pub fn build(self) -> Selection {
        Selection {
            name: self.name,
            alias: self.alias,
            arguments: self.arguments,
            nested_selections: self.nested_selections,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Selection {
    name: String,
    alias: Option<String>,
    arguments: Vec<(String, QueryValue)>,
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
    pub fn builder(name: impl Into<String>) -> SelectionBuilder {
        SelectionBuilder {
            name: name.into(),
            alias: None,
            arguments: Vec::new(),
            nested_selections: Vec::new(),
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

    pub fn is_find_one(&self) -> bool {
        self.name.starts_with("findOne")
    }

    pub fn arguments(&self) -> &[(String, QueryValue)] {
        &self.arguments
    }

    pub fn pop_argument(&mut self) -> Option<(String, QueryValue)> {
        self.arguments.pop()
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionSet<'a> {
    Single(Cow<'a, str>, Vec<QueryValue>),
    Multi(Vec<Vec<Cow<'a, str>>>, Vec<Vec<QueryValue>>),
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

    pub fn push(self, column: impl Into<Cow<'a, str>>, value: QueryValue) -> Self {
        let column = column.into();

        match self {
            Self::Single(key, mut vals) if key == column => {
                vals.push(value);
                Self::Single(key, vals)
            }
            Self::Single(key, mut vals) => {
                vals.push(value);
                Self::Multi(vec![vec![key, column]], vec![vals])
            }
            Self::Multi(mut keys, mut vals) => {
                match (keys.last_mut(), vals.last_mut()) {
                    (Some(keys), Some(vals)) if !keys.contains(&column) => {
                        keys.push(column);
                        vals.push(value);
                    }
                    _ => {
                        keys.push(vec![column]);
                        vals.push(vec![value]);
                    }
                }

                Self::Multi(keys, vals)
            }
            Self::Empty => Self::Single(column, vec![value]),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Single(_, _) => 1,
            Self::Multi(v, _) => v.len(),
            Self::Empty => 0,
        }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_, _))
    }

    pub fn is_multi(&self) -> bool {
        matches!(self, Self::Multi(_, _))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn keys(&self) -> Vec<&str> {
        match self {
            Self::Single(key, _) => vec![key.as_ref()],
            Self::Multi(keys, _) => match keys.first() {
                Some(keys) => keys.iter().map(|key| key.as_ref()).collect(),
                None => Vec::new(),
            },
            Self::Empty => Vec::new(),
        }
    }
}

pub struct In<'a> {
    selection_set: SelectionSet<'a>,
}

impl<'a> In<'a> {
    pub fn new(selection_set: SelectionSet<'a>) -> Self {
        Self { selection_set }
    }
}

impl<'a> From<In<'a>> for QueryValue {
    fn from(other: In<'a>) -> Self {
        match other.selection_set {
            SelectionSet::Multi(key_sets, val_sets) => {
                let key_vals = key_sets.into_iter().zip(val_sets.into_iter());

                let conjuctive = key_vals.fold(Conjuctive::new(), |acc, (keys, vals)| {
                    let ands = keys
                        .into_iter()
                        .zip(vals.into_iter())
                        .fold(Conjuctive::new(), |acc, (key, val)| {
                            let mut argument = IndexMap::new();
                            argument.insert(key.into_owned(), val);

                            acc.and(argument)
                        });

                    acc.or(ands)
                });

                QueryValue::from(conjuctive)
            }
            SelectionSet::Single(key, vals) => {
                let mut argument = IndexMap::new();
                argument.insert(
                    key.to_string(),
                    QueryValue::Object(vec![("in".to_owned(), QueryValue::List(vals))].into_iter().collect()),
                );

                QueryValue::Object(argument)
            }
            SelectionSet::Empty => QueryValue::Null,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Conjuctive {
    Or(Vec<Conjuctive>),
    And(Vec<Conjuctive>),
    Single(IndexMap<String, QueryValue>),
    None,
}

impl From<IndexMap<String, QueryValue>> for Conjuctive {
    fn from(map: IndexMap<String, QueryValue>) -> Self {
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

impl From<Conjuctive> for QueryValue {
    fn from(conjuctive: Conjuctive) -> Self {
        match conjuctive {
            Conjuctive::None => Self::Null,
            Conjuctive::Single(obj) => QueryValue::Object(single_to_multi_filter(obj)), // QueryValue::Object(obj),
            Conjuctive::Or(conjuctives) => {
                let conditions: Vec<QueryValue> = conjuctives.into_iter().map(QueryValue::from).collect();

                let mut map = IndexMap::new();
                map.insert("OR".to_string(), QueryValue::List(conditions));

                QueryValue::Object(map)
            }
            Conjuctive::And(conjuctives) => {
                let conditions: Vec<QueryValue> = conjuctives.into_iter().map(QueryValue::from).collect();

                let mut map = IndexMap::new();
                map.insert("AND".to_string(), QueryValue::List(conditions));

                QueryValue::Object(map)
            }
        }
    }
}

/// Syntax for single-record and multi-record queries
fn single_to_multi_filter(obj: IndexMap<String, QueryValue>) -> IndexMap<String, QueryValue> {
    let mut new_obj = IndexMap::new();

    for (key, value) in obj {
        let equality_obj = vec![("equals".to_owned(), value)].into_iter().collect();
        new_obj.insert(key, QueryValue::Object(equality_obj));
    }

    new_obj
}
