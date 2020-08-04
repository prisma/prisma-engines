use connector_interface::QueryArguments;

pub trait QueryArgumentsExt {
    /// If we need to take rows before a cursor position, then we need to reverse the order in SQL.
    fn needs_reversed_order(&self) -> bool;
}

impl QueryArgumentsExt for QueryArguments {
    fn needs_reversed_order(&self) -> bool {
        self.take.map(|t| t < 0).unwrap_or(false)
    }
}
