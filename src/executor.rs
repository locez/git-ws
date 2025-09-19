use crate::error::GitOperationError;
use crate::operations::GitOperation;
use crate::workspace::GitRepository;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;

pub struct BatchExecutor {
    concurrency_limit: usize,
}

impl BatchExecutor {
    pub fn new(concurrency_limit: usize) -> Self {
        Self { concurrency_limit }
    }

    pub async fn execute_operation(
        &self,
        operation: Arc<dyn GitOperation>,
        repositories: Vec<GitRepository>,
    ) -> Result<Vec<(String, Result<String, GitOperationError>)>, GitOperationError> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let mut handles = Vec::new();

        for repo in repositories {
            let operation = Arc::clone(&operation);
            let repo = Arc::new(repo);
            let semaphore = Arc::clone(&semaphore);

            let handle = task::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = operation.execute(repo.clone()).await;
                (repo.name.clone(), result)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.map_err(|e| {
                GitOperationError::OperationFailed(format!("Task execution failed: {}", e))
            })?;
            results.push(result);
        }

        Ok(results)
    }
}