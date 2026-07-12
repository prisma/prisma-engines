#[cfg(feature = "graphql-protocol")]
pub mod graphql;
pub mod json;

use query_core::{QueryDocument, protocol::EngineProtocol, schema::QuerySchemaRef};

#[derive(Debug)]
pub enum RequestBody {
    #[cfg(feature = "graphql-protocol")]
    Graphql(graphql::GraphqlBody),
    Json(json::JsonBody),
}

impl RequestBody {
    pub fn into_doc(self, query_schema: &QuerySchemaRef) -> crate::Result<QueryDocument> {
        match self {
            #[cfg(feature = "graphql-protocol")]
            RequestBody::Graphql(body) => body.into_doc(),
            RequestBody::Json(body) => body.into_doc(query_schema),
        }
    }

    pub fn try_from_str(val: &str, engine_protocol: EngineProtocol) -> Result<RequestBody, serde_json::Error> {
        match engine_protocol {
            #[cfg(feature = "graphql-protocol")]
            EngineProtocol::Graphql => serde_json::from_str::<graphql::GraphqlBody>(val).map(Self::from),
            EngineProtocol::Json => serde_json::from_str::<json::JsonBody>(val).map(Self::from),
        }
    }

    pub fn try_from_slice(val: &[u8], engine_protocol: EngineProtocol) -> Result<RequestBody, serde_json::Error> {
        match engine_protocol {
            #[cfg(feature = "graphql-protocol")]
            EngineProtocol::Graphql => serde_json::from_slice::<graphql::GraphqlBody>(val).map(Self::from),
            EngineProtocol::Json => serde_json::from_slice::<json::JsonBody>(val).map(Self::from),
        }
    }
}

#[cfg(feature = "graphql-protocol")]
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
