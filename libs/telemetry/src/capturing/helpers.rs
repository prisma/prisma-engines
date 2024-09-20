use tracing::Metadata;

/// Filters-in spans and events that are statically determined to be relevant for capturing
/// Dynamic filtering will be done by the [`crate::capturer::Processor`]
pub fn span_and_event_filter(meta: &Metadata<'_>) -> bool {
    if meta.fields().iter().any(|f| f.name() == "user_facing") {
        return true;
    }

    // relevant quaint connector or mongodb connector spans and events
    meta.target() == "quaint::connector::metrics" || meta.target() == "mongodb_query_connector::query"
}
