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
    pub fn spawn<F, R>(&self, task: F) -> oneshot::Receiver<R>
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
    pub fn spawn_await_result<F, R>(&self, task: F) -> impl Future<Output = R>
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

        async { receiver.await.unwrap() }
    }
}

#[cfg(test)]
#[tokio::test]
async fn simple_test() -> Result<(), anyhow::Error> {
    use std::time::Duration;
    use std::time::Instant;

    let tm = TaskManager::new();

    let start = Instant::now();

    let receiver = tm.spawn(async {
        println!("coucou");
        Instant::now()
    });

    tokio::time::sleep(Duration::from_secs(1)).await;
    let current = Instant::now();
    let task_time = receiver.await?;

    println!(
        "{} > {} ?",
        (current - start).as_millis(),
        (task_time - start).as_millis()
    );

    assert!(current > task_time);

    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn await_test() -> Result<(), anyhow::Error> {
    use std::time::Duration;
    use std::time::Instant;

    let tm = TaskManager::new();

    let start = Instant::now();

    let waiter = tm.spawn_await_result(async {
        println!("coucou");
        Instant::now()
    });

    tokio::time::sleep(Duration::from_secs(1)).await;
    let current = Instant::now();
    let task_time = waiter.await;

    println!(
        "{} > {} ?",
        (current - start).as_millis(),
        (task_time - start).as_millis()
    );

    assert!(current > task_time);

    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn sanity_check() -> Result<(), anyhow::Error> {
    use std::time::Duration;
    use std::time::Instant;

    let start = Instant::now();

    tokio::spawn(async move {
        println!("task time: {}", (Instant::now() - start).as_millis());
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("main time: {}", (Instant::now() - start).as_millis());

    Ok(())
}
