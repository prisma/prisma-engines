use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineProtocol {
    Graphql,
    Json,
}

impl Default for EngineProtocol {
    fn default() -> Self {
        Self::Graphql
    }
}

impl From<&String> for EngineProtocol {
    fn from(s: &String) -> Self {
        match s.as_str() {
            "graphql" => EngineProtocol::Graphql,
            "json" => EngineProtocol::Json,
            x => panic!("Unknown engine protocol '{x}'. Must be 'graphql' or 'json'."),
        }
    }
}
