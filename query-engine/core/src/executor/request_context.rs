use crate::protocol::EngineProtocol;

#[derive(Debug)]
struct RequestContext {
    request_now: prisma_value::PrismaValue,
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
pub(crate) fn get_request_now() -> prisma_value::PrismaValue {
    // FIXME: we want to bypass task locals if this code is executed outside of a tokio context. As
    // of this writing, it happens only in the query validation test suite.
    //
    // Eventually, this will go away when we have a plain query context reference we pass around.
    if tokio::runtime::Handle::try_current().is_err() {
        return prisma_value::PrismaValue::DateTime(chrono::Utc::now().into());
    }
    REQUEST_CONTEXT.with(|rc| rc.request_now.clone())
}

/// The engine protocol used for the whole duration of a request.
/// Use with caution to avoid creating implicit and unnecessary dependencies.
///
/// That panics if REQUEST_CONTEXT has not been set with with_request_context().
///
/// If we had a query context we carry for the entire lifetime of the query, it would belong there.
pub(crate) fn get_engine_protocol() -> EngineProtocol {
    REQUEST_CONTEXT.with(|rc| rc.engine_protocol)
}

/// Execute a future with the current "now" timestamp that can be retrieved through
/// `get_request_now()`, initializing it if necessary.
pub(crate) async fn with_request_context<F, R>(engine_protocol: EngineProtocol, fut: F) -> R
where
    F: std::future::Future<Output = R>,
{
    use chrono::{Duration, DurationRound};

    let is_set = REQUEST_CONTEXT.try_with(|_| async {}).is_ok();

    if is_set {
        fut.await
    } else {
        let timestamp_precision = Duration::milliseconds(1);
        // We round because in create operations, we select after creation and we will fail to
        // select back what we inserted if the timestamp we have is higher precision than the one
        // the database persisted.
        let dt = chrono::Utc::now().duration_round(timestamp_precision).unwrap();
        let ctx = RequestContext {
            request_now: prisma_value::PrismaValue::DateTime(dt.into()),
            engine_protocol,
        };

        REQUEST_CONTEXT.scope(ctx, fut).await
    }
}
