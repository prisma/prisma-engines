use std::time;
use tracing::Id as SpanId;

/// Gather and display timings of tracing spans.
#[derive(Default)]
pub struct TimingsLayer;

struct TimerTime(pub time::Instant, String);

impl<S> tracing_subscriber::Layer<S> for TimingsLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &SpanId,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span_ctx = ctx.span(id).unwrap();
        let mut extensions = span_ctx.extensions_mut();
        extensions.insert(TimerTime(time::Instant::now(), attrs.values().to_string()));
    }

    fn on_close(&self, id: SpanId, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ctx = ctx.span(&id).unwrap();
        let span_name = span_ctx.name();
        let mut extensions = span_ctx.extensions_mut();
        let TimerTime(start, values) = extensions.remove::<TimerTime>().unwrap();
        let elapsed = time::Instant::now().duration_since(start);
        tracing::debug!(
            span_timing_μs = elapsed.as_micros() as u32,
            "{span_name}{values}: Span closed. Elapsed: {elapsed:?}",
        );
    }
}
