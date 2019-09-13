#[derive(Debug)]
pub enum QueryGraphError {
    RuleViolation(String),
    InvalidTransformation { from: String, to: String },
}
