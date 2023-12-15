use futures::Future;
use tokio::sync::broadcast::{self};

pub struct JoinHandle<T> {
    handle: tokio::task::JoinHandle<T>,

    sx_exit: Option<broadcast::Sender<()>>,
}

impl<T> JoinHandle<T> {
    pub fn abort(&mut self) {
        if let Some(sx_exit) = self.sx_exit.as_ref() {
            sx_exit.send(()).ok();
        }

        self.handle.abort();
    }
}

pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    spawn_with_sx_exit::<T>(future, None)
}

pub fn spawn_controlled<T>(future_fn: Box<dyn FnOnce(broadcast::Receiver<()>) -> T>) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let (sx_exit, rx_exit) = tokio::sync::broadcast::channel::<()>(1);
    let future = future_fn(rx_exit);

    spawn_with_sx_exit::<T>(future, Some(sx_exit))
}

fn spawn_with_sx_exit<T>(future: T, sx_exit: Option<broadcast::Sender<()>>) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let handle = tokio::spawn(future);
    JoinHandle { handle, sx_exit }
}
