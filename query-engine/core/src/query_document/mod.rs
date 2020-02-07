//! Intermediate representation of the input document that is used by the query engine to build
//! query ASTs and validate the incoming data.
//!
//! Helps decoupling the incoming protocol layer from the query engine, i.e. allows the query engine
//! to be agnostic to the actual protocol that is used on upper layers, as long as they translate
//! to this simple intermediate representation.
//!
//! The mapping illustrated with GraphQL (GQL):
//! - There can be multiple queries and/or mutations in one GQL request, usually designated by "query / {}" or "mutation".
//! - Inside the queries / mutations are fields in GQL. In Prisma, every one of those designates exactly one `Operation` with a `Selection`.
//! - Operations are broadly divided into reading (query in GQL) or writing (mutation).
//! - The field that designates the `Operation` pretty much exactly maps to a `Selection`:
//!    - It can have arguments,
//!    - it can be aliased,
//!    - it can have a number of nested selections (selection set in GQL).
//! - Arguments contain concrete values and complex subtypes that are parsed and validated by the query builders, and then used for querying data (input types in GQL).
mod error;
mod operation;
mod parse_ast;
mod parser;
mod query_value;
mod selection;
mod transformers;

pub use error::*;
pub use operation::*;
pub use parse_ast::*;
pub use parser::*;
pub use query_value::*;
pub use selection::*;
pub use transformers::*;

pub type QueryParserResult<T> = std::result::Result<T, QueryParserError>;

#[derive(Debug)]
pub enum QueryDocument {
    Single(Operation),
    Multi(BatchDocument),
}

impl QueryDocument {
    pub fn dedup_operations(self) -> Self {
        match self {
            Self::Single(operation) => Self::Single(operation.dedup_selections()),
            _ => self,
        }
    }
}

#[derive(Debug)]
pub enum BatchDocument {
    Multi(Vec<Operation>),
    Compact(CompactedDocument),
}

impl BatchDocument {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self::Multi(operations)
    }

    fn can_compact(&self) -> bool {
        match self {
            Self::Multi(operations) => match operations.split_first() {
                Some((first, rest)) if first.is_find_one() => {
                    let mut selection1: Vec<&str> = first.nested_selections().iter().map(|s| s.name()).collect();
                    selection1.sort();

                    rest.into_iter().all(|op| {
                        let mut selection2: Vec<&str> = op.nested_selections().iter().map(|s| s.name()).collect();
                        selection2.sort();

                        op.is_find_one() && first.name() == op.name() && selection1 == selection2
                    })
                }
                _ => false,
            },
            Self::Compact(_) => false,
        }
    }

    pub fn compact(self) -> Self {
        match self {
            Self::Multi(operations) if self.can_compact() => Self::Compact(CompactedDocument::from(operations)),
            _ => self,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactedDocument {
    pub arguments: Vec<Vec<(String, QueryValue)>>,
    pub nested_selection: Vec<String>,
    pub operation: Operation,
    pub keys: Vec<String>,
    name: String,
}

impl CompactedDocument {
    pub fn single_name(&self) -> String {
        format!("findOne{}", self.name)
    }

    pub fn plural_name(&self) -> String {
        format!("findMany{}", self.name)
    }
}

/// Here be the dragons. Ay caramba!
impl From<Vec<Operation>> for CompactedDocument {
    fn from(ops: Vec<Operation>) -> Self {
        // Unpack all read queries (an enum) into a collection of selections.
        // We already took care earlier that all operations here must be reads.
        let selections: Vec<Selection> = ops
            .into_iter()
            .map(|op| op.into_read().expect("Trying to compact a write operation."))
            .collect();

        // This block creates the findMany query from the separate findOne queries.
        let selection = {
            // The name of the query should be findManyX if the first query
            // here is findOneX. We took care earlier the queries are all the
            // same. Otherwise we fail hard here.
            let mut builder = Selection::builder(selections[0].name().replacen("findOne", "findMany", 1));

            // Take the nested selection set from the first query. We took care
            // earlier that all the nested selections are the same in every
            // query. Otherwise we fail hard here.
            builder.nested_selections(selections[0].nested_selections().to_vec());

            // The query arguments are extracted here. Combine all query
            // arguments from the different queries into a one large argument.
            let selection_set = selections.iter().fold(SelectionSet::new(), |acc, selection| {
                // findOne always has only one argument. We know it must be an
                // object, otherwise this will panic.
                let obj = selection.arguments()[0]
                    .1
                    .clone()
                    .into_object()
                    .expect("Trying to compact a selection with non-object argument");

                // A "funny" trick to detect a compound key.
                match obj.values().next() {
                    // This means our query has a nested object, meaning we have
                    // a compound filter in a form of {"col1_col2": {"col1": .., "col2": ..}}
                    Some(QueryValue::Object(obj)) => obj
                        .iter()
                        .fold(acc, |acc, (key, val)| acc.push(key.clone(), val.clone())),
                    // ...or a singular argument in a form of {"col1": ..}
                    _ => obj.into_iter().fold(acc, |acc, (key, val)| acc.push(key, val)),
                }
            });

            // We must select all unique fields in the query so later on we can
            // match the right response back to the right request later on.
            for key in selection_set.keys() {
                if !builder.contains_nested_selection(key) {
                    builder.push_nested_selection(Selection::builder(key).build());
                }
            }

            // The `In` handles both cases, with singular id it'll do an `IN`
            // expression and with a compound id a combination of `AND` and `OR`.
            builder.push_argument("where", In::new(selection_set));

            if let Some(ref alias) = selections[0].alias() {
                builder.alias(alias);
            };

            builder.build()
        };

        // We want to store the original nested selections so we can filter out
        // the added unique selections from the responses if the original
        // selection set didn't have them.
        let nested_selection = selections[0]
            .nested_selections()
            .iter()
            .map(|s| s.name().to_string())
            .collect();

        // Saving the stub of the query name for later use.
        let name = selections[0].name().replacen("findOne", "", 1);

        // Convert the selections into a vector of arguments. This defines the
        // response order and how we fetch the right data from the response set.
        let arguments: Vec<Vec<(String, QueryValue)>> = selections
            .into_iter()
            .map(|mut sel| {
                let mut obj: Vec<(String, QueryValue)> = sel
                    .pop_argument()
                    .unwrap()
                    .1
                    .into_object()
                    .unwrap()
                    .into_iter()
                    .collect();

                // The trick again to detect if we have a compound key or not. (sigh)
                match obj.pop() {
                    Some((_, QueryValue::Object(obj))) => obj.into_iter().collect(),
                    Some(pair) => {
                        obj.push(pair);
                        obj
                    }
                    None => unreachable!("No arguments!")
                }
            })
            .collect();

        // The trick again to detect if we have a compound key or not. (sigh)
        // Gets the argument keys for later mapping.
        let keys = match arguments[0].iter().next() {
            Some((_, QueryValue::Object(obj))) => obj.iter().map(|(k, _)| k.to_string()).collect(),
            _ => arguments[0].iter().map(|(k, _)| k.to_string()).collect(),
        };

        Self {
            name,
            arguments,
            nested_selection,
            keys,
            operation: Operation::Read(selection),
        }
    }
}
