use std::io;
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};

use crate::worker::client::WorkerClient;

pub struct WorkerPool {
    workers: Mutex<Vec<WorkerClient>>,
    permits: Arc<Semaphore>,
}

impl WorkerPool {
    pub fn new(size: usize) -> io::Result<Self> {
        if size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "worker pool size must be greater than zero",
            ));
        }

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(WorkerClient::spawn()?);
        }

        Ok(Self {
            workers: Mutex::new(workers),
            permits: Arc::new(Semaphore::new(size)),
        })
    }

    pub async fn merge(&self, part_a: &[u8], part_b: &[u8]) -> io::Result<Vec<u8>> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| io::Error::other("worker pool was closed"))?;

        let mut worker = self
            .workers
            .lock()
            .await
            .pop()
            .ok_or_else(|| io::Error::other("worker pool had no available worker"))?;

        let result = worker.merge(part_a, part_b).await;

        match result {
            Ok(merged) => {
                self.workers.lock().await.push(worker);
                drop(permit);
                Ok(merged)
            }
            Err(err) => {
                let replacement = WorkerClient::spawn();
                if let Ok(worker) = replacement {
                    self.workers.lock().await.push(worker);
                }
                drop(permit);
                Err(err)
            }
        }
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new(16).unwrap()
    }
}