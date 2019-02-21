use crate::visitor::{Destination, Visitor};

pub struct Sqlite {
    parameters: Vec<ParameterizedValue>,
}

impl Visitor for Sqlite {
    fn visit_select(&mut self, select: &Select) {}
}
