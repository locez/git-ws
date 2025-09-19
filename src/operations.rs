use crate::error::GitOperationError;
use crate::workspace::GitRepository;
use git2::StatusOptions;
use std::sync::Arc;

// Add the async_trait attribute macro
use async_trait::async_trait;

// Add tabled for table display
use tabled::{Table, Tabled, Style, Alignment, Modify, object::Segment};

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub untracked_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
}

use std::fmt;

#[derive(Tabled)]
pub struct FileStatus {
    #[tabled(rename = "Repository")]
    repository: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "File")]
    file: String,
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

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                match entry.status() {
                    s if s.is_index_new() || s.is_wt_new() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            status: "\x1b[31mUntracked\x1b[0m".to_string(), // Red
                            file: format!("\x1b[31m{}\x1b[0m", path), // Red
                        });
                        untracked_count += 1;
                    },
                    s if s.is_wt_modified() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            status: "\x1b[33mModified\x1b[0m".to_string(), // Yellow
                            file: format!("\x1b[33m{}\x1b[0m", path), // Yellow
                        });
                        modified_count += 1;
                    },
                    s if s.is_index_modified() => {
                        file_statuses.push(FileStatus {
                            repository: repo.name.clone(),
                            status: "\x1b[32mStaged\x1b[0m".to_string(), // Green
                            file: format!("\x1b[32m{}\x1b[0m", path), // Green
                        });
                        staged_count += 1;
                    },
                    _ => {}
                }
            }
        }

        if file_statuses.is_empty() {
            Ok("Working directory clean".to_string())
        } else {
            // Group file statuses by repository and status
            use std::collections::HashMap;

            let mut repo_groups: HashMap<String, Vec<FileStatus>> = HashMap::new();
            for status in file_statuses {
                repo_groups.entry(status.repository.clone()).or_insert_with(Vec::new).push(status);
            }

            // Create a merged view of the data
            let mut merged_statuses = Vec::new();

            // Add summary row
            let summary = FileStatus {
                repository: repo.name.clone(),
                status: format!("Untracked: {}, Modified: {}, Staged: {}", untracked_count, modified_count, staged_count),
                file: "Summary".to_string(),
            };
            merged_statuses.push(summary);

            // Add grouped file statuses
            for (_repo_name, statuses) in repo_groups {
                // Group by status within each repository, keeping original FileStatus instances
                let mut status_groups: HashMap<String, Vec<FileStatus>> = HashMap::new();
                for status in statuses {
                    // Use the status text without color codes as the key
                    let status_key = status.status.replace("\x1b[31m", "").replace("\x1b[32m", "").replace("\x1b[33m", "").replace("\x1b[0m", "");
                    status_groups.entry(status_key).or_insert_with(Vec::new).push(status);
                }

                // Add rows with merged repository names and statuses
                let mut first_repo_row = true;
                for (_status_name, file_statuses) in status_groups {
                    let mut first_status_row = true;
                    for file_status in file_statuses {
                        if first_repo_row && first_status_row {
                            // First row for this repository and status - show both repo name and status
                            merged_statuses.push(file_status);
                            first_status_row = false;
                            first_repo_row = false;
                        } else if first_status_row {
                            // First row for this status but not first for repo - show empty repo name
                            merged_statuses.push(FileStatus {
                                repository: String::new(),
                                status: file_status.status,
                                file: file_status.file,
                            });
                            first_status_row = false;
                        } else {
                            // Subsequent rows - show empty repo name and status
                            merged_statuses.push(FileStatus {
                                repository: String::new(),
                                status: String::new(),
                                file: file_status.file,
                            });
                        }
                    }
                }
            }

            let table = Table::new(&merged_statuses)
                .with(Style::rounded())
                .with(Modify::new(Segment::all()).with(Alignment::left()));

            Ok(table.to_string())
        }
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