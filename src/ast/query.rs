use crate::{
    ast::Select,
    visitor::{Destination, Visitor},
};

pub enum Query {
    Select(Select),
}

impl Destination for Query {
    fn visit(&self, visitor: &mut Visitor) {
        match self {
            Query::Select(ref select) => visitor.visit_select(select),
        }
    }
}
