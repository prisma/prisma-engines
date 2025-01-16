#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects
#![allow(clippy::derive_partial_eq_without_eq)]

//! The top-level library crate for `schema-engine-wasm`.

mod api;
mod core_error;
mod state;
mod types;

// Inspired by request_handlers::load_executor::ConnectorKind
pub enum ConnectorKind<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    Rust { url: String },
    #[cfg(target_arch = "wasm32")]
    Js {
        adapter: Arc<dyn ExternalConnector>,
        active_provider: &'a str,
        _phantom: PhantomData<&'a ()>, // required for WASM target, where JS is the only variant and lifetime gets unused
    },
}

fn connector_for_provider(connector_kind: ConnectorKind<'_>) {
    match connector_kind {
        #[cfg(not(target_arch = "wasm32"))]
        ConnectorKind::Rust { .. } => {
            panic!("`core-wasm` only supports JS connectors");
        }
        #[cfg(target_arch = "wasm32")]
        ConnectorKind::Js { adapter, .. } => {}
    }
}
