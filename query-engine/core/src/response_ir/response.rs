use super::*;

/// A response can either be some `key-value` data representation
/// or an error that occured.
// #[derive(Debug)]
// pub enum Response {
//     Data(String, Item),
//     Error(ResponseError),
// }

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
