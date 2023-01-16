//! Unwraps in this module are safe because of query validation that ensures conformity to the query schema.

mod aggregations;
mod first;
mod many;
mod one;
mod related;
mod utils;

pub use aggregations::*;
pub use first::*;
pub use many::*;
pub use one::*;
pub use related::*;

use super::*;
use crate::{Query, QueryGraph, ReadQuery};

impl From<ReadQuery> for QueryGraph {
    fn from(query: ReadQuery) -> Self {
        let mut graph = QueryGraph::new();
        graph.create_node(Query::Read(query));
        graph
    }
}
