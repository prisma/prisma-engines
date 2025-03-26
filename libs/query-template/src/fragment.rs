use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Fragment {
    StringChunk(String),
    Parameter,
    ParameterTuple,
}
