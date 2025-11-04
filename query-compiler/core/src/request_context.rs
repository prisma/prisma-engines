use crate::protocol::EngineProtocol;
use query_structure::PrismaValue;

#[derive(Debug)]
struct RequestContext {
    request_now: PrismaValue,
    #[cfg(feature = "graphql-protocol")]
    engine_protocol: EngineProtocol,
}

tokio::task_local! {
    static REQUEST_CONTEXT: RequestContext;
}

/// A timestamp that should be the `NOW()` value for the whole duration of a request. So all
/// `@default(now())` and `@updatedAt` should use it.
///
/// That panics if REQUEST_CONTEXT has not been set with with_request_context().
///
/// If we had a query context we carry for the entire lifetime of the query, it would belong there.
pub(crate) fn get_request_now() -> PrismaValue {
    REQUEST_CONTEXT.try_with(|rc| rc.request_now.clone()).unwrap_or_else(|_|
            // Task local might not be set in some cases.
            // At the moment of writing, this happens only in query validation test suite.
            // In that case, we want to fall back to realtime value. On the other hand, if task local is
            // set, we want to use it, even if we are not running inside of tokio runtime (for example,
            // in WASM case)
            //
            // Eventually, this will go away when we have a plain query context reference we pass around.
            PrismaValue::DateTime(chrono::Utc::now().into()))
}

/// The engine protocol used for the whole duration of a request.
/// Use with caution to avoid creating implicit and unnecessary dependencies.
///
/// That panics if REQUEST_CONTEXT has not been set with with_request_context().
///
/// If we had a query context we carry for the entire lifetime of the query, it would belong there.
#[cfg(feature = "graphql-protocol")]
pub(crate) fn get_engine_protocol() -> EngineProtocol {
    REQUEST_CONTEXT.with(|rc| rc.engine_protocol)
}

#[cfg(not(feature = "graphql-protocol"))]
#[inline(always)]
pub(crate) fn get_engine_protocol() -> EngineProtocol {
    EngineProtocol::Json
}

pub fn with_sync_unevaluated_request_context<R>(f: impl FnOnce() -> R) -> R {
    let ctx = RequestContext {
        request_now: PrismaValue::generator_now(),
        #[cfg(feature = "graphql-protocol")]
        engine_protocol: EngineProtocol::Json,
    };
    REQUEST_CONTEXT.sync_scope(ctx, f)
}
