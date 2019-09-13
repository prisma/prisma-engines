#[derive(Debug)]
pub enum QueryGraphError {
    InvalidTransformation { from: String, to: String }
}