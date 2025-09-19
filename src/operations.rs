use crate::error::GitOperationError;
use crate::workspace::GitRepository;
use git2::{Repository, StatusOptions};
use std::sync::Arc;

// Add the async_trait attribute macro
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub untracked_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
}

#[async_trait]
pub trait GitOperation: Send + Sync {
    async fn execute(&self, repo: Arc<GitRepository>) -> Result<String, GitOperationError>;
}

pub struct StatusOperation;

#[async_trait]
impl GitOperation for StatusOperation {
    async fn execute(&self, repo: Arc<GitRepository>) -> Result<String, GitOperationError> {
        let repository = repo.open()?;
        let mut options = StatusOptions::new();
        options.include_untracked(true);

        let statuses = repository.statuses(Some(&mut options))?;

        let mut untracked = Vec::new();
        let mut modified = Vec::new();
        let mut staged = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                match entry.status() {
                    s if s.is_index_new() || s.is_wt_new() => untracked.push(path.to_string()),
                    s if s.is_wt_modified() => modified.push(path.to_string()),
                    s if s.is_index_modified() => staged.push(path.to_string()),
                    _ => {}
                }
            }
        }

        Ok(format!(
            "Status for {}:\n  Untracked: {}\n  Modified: {}\n  Staged: {}",
            repo.name,
            untracked.len(),
            modified.len(),
            staged.len()
        ))
    }
}

pub struct AddOperation {
    pub patterns: Vec<String>,
}

#[async_trait]
impl GitOperation for AddOperation {
    async fn execute(&self, repo: Arc<GitRepository>) -> Result<String, GitOperationError> {
        let repository = repo.open()?;

        let mut index = repository.index()?;

        for pattern in &self.patterns {
            index.add_path(std::path::Path::new(pattern))?;
        }

        index.write()?;

        Ok(format!("Added files to {} repository", repo.name))
    }
}

pub struct CommitOperation {
    pub message: String,
}

#[async_trait]
impl GitOperation for CommitOperation {
    async fn execute(&self, repo: Arc<GitRepository>) -> Result<String, GitOperationError> {
        let repository = repo.open()?;

        let signature = repository.signature()?;
        let mut index = repository.index()?;
        let tree_id = index.write_tree()?;
        let tree = repository.find_tree(tree_id)?;

        let commit_id = if let Ok(head) = repository.head() {
            let head_target = head.target().ok_or_else(|| {
                GitOperationError::OperationFailed("HEAD target not found".to_string())
            })?;
            let parent_commit = repository.find_commit(head_target)?;
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &self.message,
                &tree,
                &[&parent_commit],
            )?
        } else {
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &self.message,
                &tree,
                &[],
            )?
        };

        Ok(format!(
            "Committed to {} repository: {}",
            repo.name, commit_id
        ))
    }
}