use super::*;

#[derive(Debug)]
pub struct ResponseData {
    /// Top level serialization key to be used for the data.
    pub key: String,

    /// The actual response data.
    pub data: Item,
}

impl ResponseData {
    pub fn new(key: String, data: Item) -> Self {
        Self { key, data }
    }
}
