use serde::Deserialize;
use telemetry::{
    exporter::{CaptureSettings, CaptureTarget},
    Exporter, RequestId, TraceParent,
};
use tracing::Span;

#[derive(Deserialize)]
struct TraceContext<'a> {
    traceparent: Option<&'a str>,
}

pub async fn start_trace(trace_context: &str, span: &Span, exporter: &Exporter) -> Option<TraceParent> {
    let request_id = RequestId::next();
    span.record("request_id", request_id.into_u64());

    let traceparent = serde_json::from_str::<TraceContext>(trace_context)
        .ok()
        .and_then(|tc| tc.traceparent)
        .and_then(|tp| tp.parse().ok())?;

    exporter
        .start_capturing(request_id, CaptureSettings::new(CaptureTarget::Spans))
        .await;

    Some(traceparent)
}
