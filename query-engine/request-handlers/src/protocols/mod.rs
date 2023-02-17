pub mod graphql;
pub mod json;

use query_core::{protocol::EngineProtocol, schema::QuerySchemaRef, QueryDocument};

#[derive(Debug)]
pub enum RequestBody {
    Graphql(graphql::GraphqlBody),
    Json(json::JsonBody),
}

impl RequestBody {
    pub fn into_doc(self, query_schema: &QuerySchemaRef) -> crate::Result<QueryDocument> {
        match self {
            RequestBody::Graphql(body) => body.into_doc(),
            RequestBody::Json(body) => body.into_doc(query_schema),
        }
    }

    pub fn try_from_str(val: &str, engine_protocol: EngineProtocol) -> Result<RequestBody, serde_json::Error> {
        match engine_protocol {
            EngineProtocol::Graphql => serde_json::from_str::<graphql::GraphqlBody>(val).map(Self::from),
            EngineProtocol::Json => serde_json::from_str::<json::JsonBody>(val).map(Self::from),
        }
    }

    pub fn try_from_slice(val: &[u8], engine_protocol: EngineProtocol) -> Result<RequestBody, serde_json::Error> {
        match engine_protocol {
            EngineProtocol::Graphql => serde_json::from_slice::<graphql::GraphqlBody>(val).map(Self::from),
            EngineProtocol::Json => serde_json::from_slice::<json::JsonBody>(val).map(Self::from),
        }
    }

    pub fn try_as_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        match self {
            RequestBody::Graphql(body) => serde_json::to_vec(body),
            RequestBody::Json(body) => serde_json::to_vec(body),
        }
    }
}

impl From<graphql::GraphqlBody> for RequestBody {
    fn from(body: graphql::GraphqlBody) -> Self {
        Self::Graphql(body)
    }
}

impl From<json::JsonBody> for RequestBody {
    fn from(body: json::JsonBody) -> Self {
        Self::Json(body)
    }
}
