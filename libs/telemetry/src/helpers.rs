use std::collections::HashMap;

pub fn restore_remote_context_from_json_str(serialized: &str) -> opentelemetry::Context {
    // This relies on the fact that global text map propagator was installed that
    // can handle `traceparent` field (for example, `TraceContextPropagator`).
    let trace: HashMap<String, String> = serde_json::from_str(serialized).unwrap_or_default();
    opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace))
}
