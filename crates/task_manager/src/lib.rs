//! Task Manager ensuring a task will be executed in case the execution is dependant on the execution of another
//! task that can fail, be dropped etc... Typical use case is to ensure a write in DB event if the client disconnects.

use std::pin::Pin;
use std::sync::Arc;

use futures::Future;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

#[derive(Clone, Debug)]
pub struct TaskManager {
    sender: Arc<mpsc::UnboundedSender<Pin<Box<dyn Future<Output = ()> + Send>>>>,
    _worker_handle: Arc<JoinHandle<()>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let worker = tokio::task::spawn(async move {
            while let Some(task) = receiver.recv().await {
                tokio::spawn(task);
            }
        });

        Self {
            sender: Arc::new(sender),
            _worker_handle: Arc::new(worker),
        }
    }

    /// Use this function to "push and forget" or if you want to await for the result by yourself.
    pub async fn spawn<F, R>(&self, task: F) -> oneshot::Receiver<R>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();

        let wrapped = async move {
            let r = task.await;

            let _ = sender.send(r);
        };

        let _ = self
            .sender
            .send(Box::pin(wrapped))
            .map_err(|e| format!("{e}"))
            .expect("Can't send task");

        receiver
    }

    /// Use this function if you need the result of the future directly without ugly `flatten` or double await.
    pub async fn spawn_await_result<F, R>(&self, task: F) -> R
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();

        let wrapped = async move {
            let r = task.await;

            let _ = sender.send(r);
        };

        let _ = self
            .sender
            .send(Box::pin(wrapped))
            .map_err(|e| format!("{e}"))
            .expect("Can't send task");

        receiver.await.unwrap()
    }
}
