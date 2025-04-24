use psl::ConnectorRegistry;
use quaint::connector::ConnectionInfo;
use query_compiler::{CompileError, Expression, TranslateError};
use query_core::{ArgumentValue, BatchDocument, QueryDocument, QueryGraphBuilderError, protocol::EngineProtocol};
use request_handlers::{HandlerError, RequestBody};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::params::{AdapterProvider, JsConnectionInfo};

const CONNECTOR_REGISTRY: ConnectorRegistry<'_> = &[
    #[cfg(feature = "postgresql")]
    psl::builtin_connectors::POSTGRES,
    #[cfg(feature = "mysql")]
    psl::builtin_connectors::MYSQL,
    #[cfg(feature = "sqlite")]
    psl::builtin_connectors::SQLITE,
];

#[wasm_bindgen]
extern "C" {
    /// This function registers the reason for a Wasm panic via the
    /// JS function `globalThis.PRISMA_WASM_PANIC_REGISTRY.set_message()`
    #[wasm_bindgen(js_namespace = ["global", "PRISMA_WASM_PANIC_REGISTRY"], js_name = "set_message")]
    fn prisma_set_wasm_panic_message(s: &str);
}

/// Registers a singleton panic hook that will register the reason for the Wasm panic in JS.
/// Without this, the panic message would be lost: you'd see `RuntimeError: unreachable` message in JS,
/// with no reference to the Rust function and line that panicked.
/// This function should be manually called before any other public function in this module.
/// Note: no method is safe to call after a panic has occurred.
fn register_panic_hook() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();

    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let message = &info.to_string();
            prisma_set_wasm_panic_message(message);
        }));
    });
}

#[derive(Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[tsify(from_wasm_abi)]
pub struct QueryCompilerParams {
    // TODO: support multiple datamodels
    datamodel: String,
    provider: AdapterProvider,
    connection_info: JsConnectionInfo,
}

#[wasm_bindgen]
pub struct QueryCompiler {
    schema: Arc<schema::QuerySchema>,
    connection_info: ConnectionInfo,
    protocol: EngineProtocol,
}

#[wasm_bindgen]
impl QueryCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(params: QueryCompilerParams) -> Result<QueryCompiler, wasm_bindgen::JsError> {
        let QueryCompilerParams {
            datamodel,
            provider,
            connection_info,
        } = params;

        // Note: if we used `psl::validate`, we'd add ~1MB to the Wasm artifact (before gzip).
        let schema = Arc::new(psl::parse_without_validation(datamodel.into(), CONNECTOR_REGISTRY));
        let schema = Arc::new(schema::build(schema, true));

        tracing::info!(git_hash = env!("GIT_HASH"), "Starting query-compiler-wasm");
        register_panic_hook();

        Ok(Self {
            schema,
            connection_info: ConnectionInfo::External(connection_info.into_external_connection_info(provider)),
            protocol: EngineProtocol::Json,
        })
    }

    #[wasm_bindgen]
    pub fn compile(&self, request: String) -> Result<String, JsCompileError> {
        let request = RequestBody::try_from_str(&request, self.protocol)?;
        let QueryDocument::Single(op) = request.into_doc(&self.schema)? else {
            return Err(JsCompileError::plain("Unexpected batch request"));
        };
        let plan = query_compiler::compile(&self.schema, op, &self.connection_info)?;
        Ok(serde_json::to_string(&plan)?)
    }

    #[wasm_bindgen(js_name = compileBatch)]
    pub fn compile_batch(&self, request: String) -> Result<BatchResponse, JsCompileError> {
        let request = RequestBody::try_from_str(&request, self.protocol)?;
        match request.into_doc(&self.schema)? {
            QueryDocument::Single(op) => {
                let plan = query_compiler::compile(&self.schema, op, &self.connection_info)?;
                Ok(BatchResponse::Multi { plans: vec![plan] })
            }
            QueryDocument::Multi(batch) => match batch.compact(&self.schema) {
                BatchDocument::Multi(operations, _) => {
                    let plans = operations
                        .into_iter()
                        .map(|op| query_compiler::compile(&self.schema, op, &self.connection_info))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(BatchResponse::Multi { plans })
                }
                BatchDocument::Compact(compacted) => {
                    let expect_non_empty = compacted.throw_on_empty();
                    let plan = query_compiler::compile(&self.schema, compacted.operation, &self.connection_info)?;
                    Ok(BatchResponse::Compacted {
                        plan,
                        arguments: compacted.arguments,
                        nested_selection: compacted.nested_selection,
                        keys: compacted.keys,
                        expect_non_empty,
                    })
                }
            },
        }
    }
}

#[derive(Serialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase")]
#[tsify(into_wasm_abi, hashmap_as_object)]
pub enum BatchResponse {
    Multi {
        plans: Vec<Expression>,
    },
    #[serde(rename_all = "camelCase")]
    Compacted {
        plan: Expression,
        arguments: Vec<HashMap<String, ArgumentValue>>,
        nested_selection: Vec<String>,
        keys: Vec<String>,
        expect_non_empty: bool,
    },
}

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi, hashmap_as_object)]
pub struct JsCompileError {
    message: String,
    code: Option<String>,
    meta: Option<serde_json::Value>,
}

impl JsCompileError {
    pub fn plain(message: impl Into<String>) -> Self {
        JsCompileError {
            message: message.into(),
            code: None,
            meta: None,
        }
    }
}

impl From<CompileError> for JsCompileError {
    fn from(value: CompileError) -> Self {
        match value {
            CompileError::GraphBuildError(QueryGraphBuilderError::QueryParserError(error))
            | CompileError::TranslateError(TranslateError::GraphBuildError(
                QueryGraphBuilderError::QueryParserError(error),
            )) => JsCompileError {
                message: error.message().into(),
                code: Some(error.kind().code().into()),
                meta: serde_json::to_value(&error).ok(),
            },
            _ => JsCompileError {
                message: value.to_string(),
                code: None,
                meta: None,
            },
        }
    }
}

impl From<serde_json::Error> for JsCompileError {
    fn from(error: serde_json::Error) -> Self {
        JsCompileError {
            message: format!("JSON Error: {error}"),
            code: None,
            meta: None,
        }
    }
}

impl From<HandlerError> for JsCompileError {
    fn from(error: HandlerError) -> Self {
        JsCompileError {
            message: format!("{error}"),
            code: None,
            meta: None,
        }
    }
}
