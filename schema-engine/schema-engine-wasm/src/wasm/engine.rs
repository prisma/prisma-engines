#![allow(dead_code)]
#![allow(unused_variables)]

use driver_adapters::{adapter_factory_from_js, JsObject};
use json_rpc::types::*;
use psl::{ConnectorRegistry, ValidatedSchema};
use quaint::connector::ExternalConnectorFactory;
use serde::Deserialize;
use sql_schema_connector::SqlSchemaConnector;
use std::sync::Arc;
use tsify_next::Tsify;
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
    adapter_factory: Arc<dyn ExternalConnectorFactory>,
    sql_schema_connector: SqlSchemaConnector,
}

// 1. One SchemaEngine object that reads 1 schema only and exposes methods that actually make use of such schema
// 2. A bunch of free functions (e.g., diff, version) that either don't rely on any schema,
//    or accept multiple schemas as input.

#[wasm_bindgen]
impl SchemaEngine {
    #[wasm_bindgen(constructor)]
    pub async fn new(adapter: JsObject) -> Result<SchemaEngine, wasm_bindgen::JsError> {
        let adapter_factory = Arc::new(adapter_factory_from_js(adapter));
        let adapter = Arc::new(adapter_factory.connect().await?);

        let sql_schema_connector = SqlSchemaConnector::new_from_external(adapter).await?;

        tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema-engine-wasm");
        register_panic_hook();

        Ok(Self {
            adapter_factory,
            sql_schema_connector,
        })
    }

    /// Debugging method that only panics, for tests.
    #[wasm_bindgen(js_name = "debugPanic")]
    pub fn debug_panic(&self) {
        panic!("This is the debugPanic artificial panic")
    }
}
