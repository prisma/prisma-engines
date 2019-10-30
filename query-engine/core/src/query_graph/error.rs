#[derive(Debug)]
pub enum QueryGraphError {
    /// Error expressing an error in constructing the query graph.
    /// Usually hints at a logic error, not a user input error.
    InvarianceViolation(String),

    /// Error expressing an invalid transformation done on graph nodes.
    InvalidNodeTransformation { from: String, to: String },
}
