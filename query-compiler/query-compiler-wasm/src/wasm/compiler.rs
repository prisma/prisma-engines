#![allow(dead_code)]
#![allow(unused_variables)]

use driver_adapters::JsObject;
use psl::ConnectorRegistry;
use quaint::connector::{ConnectionInfo, ExternalConnector};
use query_core::protocol::EngineProtocol;
use request_handlers::RequestBody;
use serde::Deserialize;
use std::sync::Arc;
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

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

/// The main query engine used by JS
#[wasm_bindgen]
pub struct QueryCompiler {
    schema: Arc<schema::QuerySchema>,
    adapter: Arc<dyn ExternalConnector>,
    protocol: EngineProtocol,
}

#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct QueryCompilerParams {
    // TODO: support multiple datamodels
    datamodel: String,
}

#[wasm_bindgen]
impl QueryCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(params: QueryCompilerParams, adapter: JsObject) -> Result<QueryCompiler, wasm_bindgen::JsError> {
        let QueryCompilerParams { datamodel, .. } = params;

        // Note: if we used `psl::validate`, we'd add ~1MB to the Wasm artifact (before gzip).
        let schema = Arc::new(psl::parse_without_validation(datamodel.into(), CONNECTOR_REGISTRY));
        let schema = Arc::new(schema::build(schema, true));
        let adapter = Arc::new(driver_adapters::from_js(adapter));

        tracing::info!(git_hash = env!("GIT_HASH"), "Starting query-compiler-wasm");
        register_panic_hook();

        Ok(Self {
            schema,
            adapter,
            protocol: EngineProtocol::Json,
        })
    }

    #[wasm_bindgen]
    pub async fn compile(
        &self,
        request: String,
        _human_readable: bool, // ignored on wasm to not compile it in
    ) -> Result<String, wasm_bindgen::JsError> {
        let request = RequestBody::try_from_str(&request, self.protocol)?;
        let query_doc = request.into_doc(&self.schema)?;

        let connection_info = ConnectionInfo::External(self.adapter.get_connection_info().await?);

        let plan = query_compiler::compile(&self.schema, query_doc, &connection_info)?;
        Ok(serde_json::to_string(&plan)?)
    }
}
