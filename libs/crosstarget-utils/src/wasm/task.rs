use futures::Future;
use tokio::sync::{
    broadcast::{self},
    oneshot::{self},
};

// Wasm-compatible alternative to `tokio::task::JoinHandle<T>`.
// `pin_project` enables pin-projection and a `Pin`-compatible implementation of the `Future` trait.
#[pin_project::pin_project]
pub struct JoinHandle<T> {
    #[pin]
    receiver: oneshot::Receiver<T>,

    sx_exit: Option<broadcast::Sender<()>>,
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, oneshot::error::RecvError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        // the `self.project()` method is provided by the `pin_project` macro
        core::pin::Pin::new(&mut self.receiver).poll(cx)
    }
}

impl<T> JoinHandle<T> {
    pub fn abort(&mut self) {
        if let Some(sx_exit) = self.sx_exit.as_ref() {
            sx_exit.send(()).ok();
        }
    }
}

pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: Send + 'static,
{
    spawn_with_sx_exit::<T>(future, None)
}

pub fn spawn_controlled<T>(future_fn: Box<dyn FnOnce(broadcast::Receiver<()>) -> T>) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: Send + 'static,
{
    let (sx_exit, rx_exit) = tokio::sync::broadcast::channel::<()>(1);
    let future = future_fn(rx_exit);
    spawn_with_sx_exit::<T>(future, Some(sx_exit))
}

fn spawn_with_sx_exit<T>(future: T, sx_exit: Option<broadcast::Sender<()>>) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: Send + 'static,
{
    let (sender, receiver) = oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let result = future.await;
        sender.send(result).ok();
    });

    JoinHandle { receiver, sx_exit }
}
