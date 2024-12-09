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

pub async fn start_trace(
    request_id: RequestId,
    trace_context: &str,
    span: &Span,
    exporter: &Exporter,
) -> Option<TraceParent> {
    span.record("request_id", request_id.into_u64());

    let traceparent = serde_json::from_str::<TraceContext>(trace_context)
        .ok()
        .and_then(|tc| tc.traceparent)
        .and_then(|tp| tp.parse().ok());

    if traceparent.is_some() {
        exporter
            .start_capturing(request_id, CaptureSettings::new(CaptureTarget::Spans))
            .await;
    }

    traceparent
}
