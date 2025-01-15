use futures::Future;

/// Allow asynchronous futures to be sent across threads, solving the following error on `wasm32-*` targets:
///
///  ```text
/// future cannot be sent between threads safely
/// the trait `Send` is not implemented for `dyn Future<Output = std::result::Result<u32, js_sys::Error>>`.
/// ```
///
/// This wrapper is used by both the Napi.rs and Wasm implementation of `driver-adapters`, but is only really
/// needed because `wasm-bindgen` does not implement `Send` for `Future`, and most of the codebase
/// uses `#[async_trait]`, which requires `Send` on the future returned by `async fn` declarations.
///
/// In fact, `UnsafeFuture<F>` safely implements `Send` if `F` implements `Future + Send`, which is the case
/// with Napi.rs, but not with Wasm.
///
/// See: https://github.com/rustwasm/wasm-bindgen/issues/2409#issuecomment-820750943
#[pin_project::pin_project]
pub struct UnsafeFuture<F: Future>(#[pin] pub F);

impl<F: Future> Future for UnsafeFuture<F> {
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        // the `self.project()` method is provided by the `pin_project` macro
        let future: std::pin::Pin<&mut F> = self.project().0;
        future.poll(cx)
    }
}

#[cfg(target_arch = "wasm32")]
unsafe impl<F: Future> Send for UnsafeFuture<F> {}
