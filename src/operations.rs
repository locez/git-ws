use crate::error::GitOperationError;
use crate::workspace::GitRepository;
use colored::Colorize;
use git2::StatusOptions;
use std::sync::Arc;

// Add the async_trait attribute macro
use async_trait::async_trait;

// Add tabled for table display
use serde_json;
use tabled::Tabled;

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub untracked_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
}

use std::fmt;

#[derive(Tabled, serde::Serialize, serde::Deserialize, Clone)]
pub struct FileStatus {
    #[tabled(rename = "Repository")]
    pub repository: String,
    #[tabled(rename = "Summary")]
    pub summary: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "File")]
    pub file: String,
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let colored_status = match self.status.as_str() {
            "Untracked" => format!("\x1b[31m{}\x1b[0m", self.status), // Red
            "Modified" => format!("\x1b[33m{}\x1b[0m", self.status),  // Yellow
            "Staged" => format!("\x1b[32m{}\x1b[0m", self.status),    // Green
            _ => self.status.clone(),
        };
        write!(f, "{} {} {}", self.repository, colored_status, self.file)
    }
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

        let mut file_statuses = Vec::new();
        let mut untracked_count = 0;
        let mut modified_count = 0;
        let mut staged_count = 0;

        if statuses.is_empty() {
            file_statuses.push(FileStatus {
                repository: repo.name.clone(),
                summary: "Clean".to_string(),
                status: "\x1b[32mClean\x1b[0m".to_string(), // Green
                file: "".to_string(),
            });
            return Ok(serde_json::to_string(&file_statuses)
                .map_err(|e| GitOperationError::OperationFailed(e.to_string()))?);
        }

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                match entry.status() {
                    s if s.is_index_new() || s.is_wt_new() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            summary: "".to_string(),
                            status: "\x1b[31mUntracked\x1b[0m".to_string(), // Red
                            file: format!("\x1b[31m{}\x1b[0m", path),       // Red
                        });
                        untracked_count += 1;
                    }
                    s if s.is_wt_modified() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            summary: "".to_string(),
                            status: "Modified".yellow().to_string(), // Yellow
                            file: path.yellow().to_string(),         // Yellow
                        });
                        modified_count += 1;
                    }
                    s if s.is_index_modified() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            summary: "".to_string(),
                            status: "\x1b[32mStaged\x1b[0m".to_string(), // Green
                            file: format!("\x1b[32m{}\x1b[0m", path),    // Green
                        });
                        staged_count += 1;
                    }
                    _ => {}
                }
            }
        }

        // Return structured data instead of a formatted table

        for status in file_statuses.iter_mut() {
            status.summary = format!(
                "Untracked: {}\nModified: {}\nStaged: {}",
                untracked_count, modified_count, staged_count
            );
        }
        println!("{}", serde_json::to_string_pretty(&file_statuses).unwrap());

        Ok(serde_json::to_string(&file_statuses)
            .map_err(|e| GitOperationError::OperationFailed(e.to_string()))?)
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
