//! This module provides a unified interface for spawning asynchronous tasks, regardless of the target platform.

pub use arch::{spawn, JoinHandle};
use futures::Future;

// On native targets, `tokio::spawn` spawns a new asynchronous task.
#[cfg(not(target_arch = "wasm32"))]
mod arch {
    use super::*;

    pub type JoinHandle<T> = tokio::task::JoinHandle<T>;

    pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(future)
    }
}

// On Wasm targets, `wasm_bindgen_futures::spawn_local` spawns a new asynchronous task.
#[cfg(target_arch = "wasm32")]
mod arch {
    use super::*;
    use tokio::sync::oneshot::{self};

    // Wasm-compatible alternative to `tokio::task::JoinHandle<T>`.
    // `pin_project` enables pin-projection and a `Pin`-compatible implementation of the `Future` trait.
    pub struct JoinHandle<T>(oneshot::Receiver<T>);

    impl<T> Future for JoinHandle<T> {
        type Output = Result<T, oneshot::error::RecvError>;

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            // the `self.project()` method is provided by the `pin_project` macro
            core::pin::Pin::new(&mut self.0).poll(cx)
        }
    }

    impl<T> JoinHandle<T> {
        pub fn abort(&mut self) {
            // abort is noop on Wasm targets
        }
    }

    pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            let result = future.await;
            sender.send(result).ok();
        });
        JoinHandle(receiver)
    }
}
