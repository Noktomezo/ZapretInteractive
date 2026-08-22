use anyhow::{Context as _, Result};
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::Runtime;

pub static TOKIO: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("zapret-interactive-tokio")
        .build()
        .expect("Failed to initialize background Tokio runtime")
});

pub fn spawn_tokio<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TOKIO.spawn(future)
}

pub async fn run_tokio<F, T>(future: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    spawn_tokio(future)
        .await
        .context("Tokio background task panicked or was cancelled")?
}
