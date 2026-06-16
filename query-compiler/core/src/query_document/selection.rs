use crate::{ArgumentValue, ArgumentValueObject};
use indexmap::IndexMap;
use itertools::Itertools;
use schema::constants::filters;
use std::iter;

pub type SelectionArgument = (String, ArgumentValue);

#[derive(Clone)]
pub struct Selection {
    name: String,
    alias: Option<String>,
    arguments: Vec<(String, ArgumentValue)>,
    nested_selections: Vec<Selection>,
    nested_exclusions: Option<Vec<Exclusion>>,
}

impl std::fmt::Debug for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut strct = f.debug_struct("Selection");

        strct.field("name", &self.name);

        if self.alias.is_some() {
            strct.field("alias", &self.alias);
        }

        if !self.arguments().is_empty() {
            strct.field("arguments", &self.arguments);
        }

        if !self.nested_selections().is_empty() {
            strct.field("nested_selections", &self.nested_selections);
        }

        if self.nested_exclusions.is_some() {
            strct.field("nested_exclusions", &self.nested_exclusions);
        }

        strct.finish()
    }
}

/// Represents a field that's excluded.
#[derive(Debug, Clone)]
pub struct Exclusion {
    pub name: String,
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
            nested_exclusions: None,
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

    pub fn argument<'a>(&'a self, name: &str) -> Option<&'a ArgumentValue> {
        self.arguments.iter().find(|(k, _)| k == name).map(|(_, v)| v)
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
        // If a selection with the same name and alias already exists, replace it.
        // We do that to avoid duplicates in the selection set. This can happen in the JSON protocol when
        // a wildcard selection and an explicit selection set are combined. eg: `$scalars: true, id: true`
        // This case should technically never happen atm, but it's a safety net which ensures we always keep the last one that we encounter.
        match self
            .nested_selections
            .iter()
            .find_position(|sel| sel.name() == selection.name() && sel.alias() == selection.alias())
        {
            Some((idx, _)) => {
                self.nested_selections[idx] = selection;
            }
            None => self.nested_selections.push(selection),
        }
    }

    pub fn push_nested_exclusion(&mut self, name: impl Into<String>) {
        let exclusion = Exclusion { name: name.into() };

        match self.nested_exclusions {
            Some(ref mut exclusions) => exclusions.push(exclusion),
            None => self.nested_exclusions = Some(vec![exclusion]),
        }
    }

    pub fn remove_nested_selection(&mut self, name: &str) {
        self.nested_selections.retain(|sel| sel.name() != name);
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

    pub fn nested_exclusions(&self) -> Option<&[Exclusion]> {
        self.nested_exclusions.as_deref()
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

    pub fn has_no_keys(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get_single_key(&self) -> Option<&(String, ArgumentValue)> {
        self.0.first()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SelectionSet {
    Single(QuerySingle),
    Many(Vec<QueryFilters>),
    #[default]
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuerySingle(String, Vec<ArgumentValue>);

impl QuerySingle {
    /// Attempt at building a single query filter from multiple query filters.
    /// Returns `None` if the input is empty, if any of the query filters carry
    /// more than one key, or if any of them carry zero keys.
    ///
    /// The zero-keys case fires when the TS client batches concurrent
    /// `findUnique` calls and at least one of them has an `undefined` unique
    /// value: validation catches that on the single-query path but the
    /// batched path strips `undefined` keys and arrives here with an empty
    /// `QueryFilters`. Falling through to `first.get_single_key().unwrap()`
    /// in that state panicked on every batched concurrent call and took out
    /// every other request sharing the batch window. See
    /// https://github.com/prisma/prisma/issues/29642.
    pub fn new(query_filters: &[QueryFilters]) -> Option<Self> {
        if query_filters.is_empty() {
            return None;
        }

        if query_filters
            .iter()
            .any(|query_filters| query_filters.has_many_keys() || query_filters.has_no_keys())
        {
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

#[derive(Debug, PartialEq, Default)]
pub enum Conjuctive {
    Or(Vec<Conjuctive>),
    And(Vec<Conjuctive>),
    Single(ArgumentValueObject),
    #[default]
    None,
}

impl From<ArgumentValueObject> for Conjuctive {
    fn from(map: ArgumentValueObject) -> Self {
        Self::Single(map)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArgumentValue;

    fn make_filter(pairs: &[(&str, i64)]) -> QueryFilters {
        QueryFilters(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), ArgumentValue::int(*v)))
                .collect(),
        )
    }

    // Regression for https://github.com/prisma/prisma/issues/29642.
    //
    // When the TS client batches concurrent `findUnique` calls and at least
    // one has an `undefined` unique value, the value is stripped and the
    // batched path arrives at `QuerySingle::new` with one or more empty
    // `QueryFilters`. Before the fix, `has_many_keys()` was the only
    // pre-condition checked, so empty filters fell through to
    // `first.get_single_key().unwrap()` and panicked, taking out every other
    // request sharing the batch window as collateral. Now they return None,
    // which `SelectionSet::new` maps to `SelectionSet::Many(filters)` (or
    // `SelectionSet::Empty` for the all-empty case) without panicking.
    #[test]
    fn query_single_new_returns_none_for_empty_filter_instead_of_panicking() {
        let filters = vec![QueryFilters(Vec::new())];
        assert!(QuerySingle::new(&filters).is_none());
    }

    #[test]
    fn query_single_new_returns_none_when_any_filter_in_batch_is_empty() {
        // The bug surfaces specifically when *concurrent* batched callers
        // mix valid + invalid filters; the unwrap fires on the first empty
        // one regardless of position.
        let filters = vec![
            make_filter(&[("id", 1)]),
            QueryFilters(Vec::new()), // the bad one
            make_filter(&[("id", 2)]),
        ];
        assert!(QuerySingle::new(&filters).is_none());
    }

    #[test]
    fn query_filters_has_no_keys_distinguishes_empty_from_populated() {
        assert!(QueryFilters(Vec::new()).has_no_keys());
        assert!(!make_filter(&[("id", 1)]).has_no_keys());
    }

    #[test]
    fn query_single_new_still_works_for_well_formed_single_key_batch() {
        // Behaviour-preservation sanity check — the fix narrows what's
        // accepted, but valid single-key batches must continue to coalesce.
        let filters = vec![make_filter(&[("id", 1)]), make_filter(&[("id", 2)])];
        let single = QuerySingle::new(&filters).expect("valid batch should coalesce");
        assert_eq!(single.0, "id");
        assert_eq!(single.1.len(), 2);
    }
}
