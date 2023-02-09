use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineProtocol {
    Graphql,
    Json,
}

impl EngineProtocol {
    /// Returns `true` if the engine protocol is [`Json`].
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// Returns `true` if the engine protocol is [`Graphql`].
    pub fn is_graphql(&self) -> bool {
        matches!(self, Self::Graphql)
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
