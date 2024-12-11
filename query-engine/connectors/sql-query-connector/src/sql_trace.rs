use quaint::ast::{Delete, Insert, Select, Update};
use telemetry::TraceParent;

pub trait SqlTraceComment: Sized {
    fn add_traceparent(self, traceparent: Option<TraceParent>) -> Self;
}

macro_rules! sql_trace {
    ($what:ty) => {
        impl SqlTraceComment for $what {
            fn add_traceparent(self, traceparent: Option<TraceParent>) -> Self {
                let Some(traceparent) = traceparent else {
                    return self;
                };

                if traceparent.sampled() {
                    self.comment(format!("traceparent='{traceparent}'"))
                } else {
                    self
                }
            }
        }
    };
}

sql_trace!(Insert<'_>);

sql_trace!(Update<'_>);

sql_trace!(Delete<'_>);

sql_trace!(Select<'_>);
