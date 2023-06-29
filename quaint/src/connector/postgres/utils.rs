use serde::{Deserialize, Serialize};

pub fn generate_shape_and_tag(shape: &str) -> (String, String) {
    let shape = shape.to_owned();

    let mut hasher = sha1::Sha1::new();
    hasher.update(shape.as_bytes());

    let tag = hasher.digest().to_string();

    (shape, tag)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SubmittedQueryInfo {
    pub raw_query: String,
    pub tag: String,
    pub prisma_query: String,
}
