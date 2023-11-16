use futures::Future;

// Allow asynchronous futures to be sent safely across threads, solving the following error:
//
//  ```text
// future cannot be sent between threads safely
// the trait `Send` is not implemented for `dyn Future<Output = std::result::Result<u32, js_sys::Error>>`.
// ```
//
// See: https://github.com/rustwasm/wasm-bindgen/issues/2409#issuecomment-820750943
#[pin_project::pin_project]
pub struct SendFuture<F: Future>(#[pin] pub F);

unsafe impl<F: Future> Send for SendFuture<F> {}

impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        // the `self.project()` method is provided by the `pin_project` macro
        let future: std::pin::Pin<&mut F> = self.project().0;
        future.poll(cx)
    }
}
