use serde::Deserialize;
use telemetry::{
    Exporter, RequestId, TraceParent,
    exporter::{CaptureSettings, CaptureTarget},
};
use tracing::Span;

use crate::error::ApiError;

#[derive(Deserialize)]
struct TraceContext<'a> {
    traceparent: Option<&'a str>,
}

pub async fn start_trace(
    request_id: &str,
    trace_context: &str,
    span: &Span,
    exporter: &Exporter,
) -> Result<Option<TraceParent>, ApiError> {
    let request_id = request_id
        .parse::<RequestId>()
        .map_err(|_| ApiError::Decode("invalid request id".into()))?;

    span.record("request_id", request_id.into_u64());

    let traceparent = serde_json::from_str::<TraceContext>(trace_context)
        .ok()
        .and_then(|tc| tc.traceparent)
        .and_then(|tp| tp.parse::<TraceParent>().ok());

    if let Some(traceparent) = traceparent
        && traceparent.sampled()
    {
        exporter
            .start_capturing(request_id, CaptureSettings::new(CaptureTarget::Spans))
            .await;
    }

    Ok(traceparent)
}
