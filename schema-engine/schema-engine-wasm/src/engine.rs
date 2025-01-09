#![allow(dead_code)]
#![allow(unused_variables)]

use driver_adapters::JsObject;
use psl::{ConnectorRegistry, ValidatedSchema};
use quaint::connector::ExternalConnector;
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
pub struct SchemaEngine {
    schema: ValidatedSchema,
    adapter: Arc<dyn ExternalConnector>,
}

#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct SchemaEngineParams {
    // TODO: support multiple datamodels
    datamodel: String,
}

#[wasm_bindgen]
impl SchemaEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(params: SchemaEngineParams, adapter: JsObject) -> Result<SchemaEngine, wasm_bindgen::JsError> {
        let SchemaEngineParams { datamodel, .. } = params;

        // Note: if we used `psl::validate`, we'd add ~1MB to the Wasm artifact (before gzip).
        let schema = psl::parse_without_validation(datamodel.into(), CONNECTOR_REGISTRY);
        let js_queryable = Arc::new(driver_adapters::from_js(adapter));

        tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema-engine-wasm");
        register_panic_hook();

        Ok(Self {
            schema,
            adapter: js_queryable,
        })
    }

    /// Debugging method that only panics, for tests.
    #[wasm_bindgen(js_name = "debugPanic")]
    pub fn debug_panic(&self) {
        panic!("This is the debugPanic artificial panic")
    }

    /// Return the database version as a string.
    #[wasm_bindgen]
    pub async fn version(&self) -> Result<Option<String>, wasm_bindgen::JsError> {
        Err(wasm_bindgen::JsError::new("Not yet available."))
    }

    /// Reset a database to an empty state (no data, no schema).
    pub async fn reset(&self) -> Result<(), wasm_bindgen::JsError> {
        Err(wasm_bindgen::JsError::new("Not yet available."))
    }
}
