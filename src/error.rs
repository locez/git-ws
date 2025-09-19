use std::fmt;

#[derive(Debug)]
pub enum GitWsError {
    GitError(git2::Error),
    IoError(std::io::Error),
    RepositoryNotFound(String),
    OperationFailed(String),
}

impl fmt::Display for GitWsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitWsError::GitError(e) => write!(f, "Git error: {}", e),
            GitWsError::IoError(e) => write!(f, "IO error: {}", e),
            GitWsError::RepositoryNotFound(name) => write!(f, "Repository not found: {}", name),
            GitWsError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for GitWsError {}

impl From<git2::Error> for GitWsError {
    fn from(error: git2::Error) -> Self {
        GitWsError::GitError(error)
    }
}

impl From<std::io::Error> for GitWsError {
    fn from(error: std::io::Error) -> Self {
        GitWsError::IoError(error)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitOperationError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}