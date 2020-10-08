//! Unwraps in this module are safe because of query validation that ensures conformity to the query schema.

mod aggregate;
mod first;
mod many;
mod one;
mod related;
mod utils;

pub use aggregate::*;
pub use first::*;
pub use many::*;
pub use one::*;
pub use related::*;

use super::*;
use crate::{Query, QueryGraph, ReadQuery};

impl Into<QueryGraph> for ReadQuery {
    fn into(self) -> QueryGraph {
        let mut graph = QueryGraph::new();
        graph.create_node(Query::Read(self));
        graph
    }
}
