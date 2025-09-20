use git2::{Repository, Error as Git2Error};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct GitRepository {
    pub path: PathBuf,
    pub name: String,
}

impl GitRepository {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self { path, name }
    }

    pub fn open(&self) -> Result<Repository, Git2Error> {
        Repository::open(&self.path)
    }
}

pub struct Workspace {
    pub root_path: PathBuf,
    pub repositories: HashMap<String, GitRepository>,
}

impl Workspace {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            repositories: HashMap::new(),
        }
    }

    pub async fn discover_repositories(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.repositories.clear();
        self.discover_repositories_recursive(self.root_path.clone()).await?;
        Ok(())
    }

    async fn discover_repositories_recursive(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.discover_repositories_recursive_helper(path).await
    }

    fn discover_repositories_recursive_helper(&mut self, path: PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send>>> + Send + '_>> {
        Box::pin(async move {
            // Check if current path is a git repository
            if path.join(".git").exists() {
                let repo = GitRepository::new(path.clone());
                self.repositories.insert(repo.name.clone(), repo);
                // Don't traverse deeper into .git directories
                // return Ok(());
            }

            // Skip hidden directories except .git
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') && name != ".git" {
                    return Ok(());
                }
            }

            // Read directory entries
            let mut entries = match fs::read_dir(&path).await {
                Ok(entries) => entries,
                Err(_) => return Ok(()), // If we can't read the directory, skip it
            };

            let mut entries_vec = Vec::new();
            while let Some(entry) = entries.next_entry().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)? {
                entries_vec.push(entry);
            }

            // Process each directory entry recursively
            for entry in entries_vec {
                let entry_path = entry.path();

                // Only process directories
                if entry_path.is_dir() {
                    self.discover_repositories_recursive_helper(entry_path).await?;
                }
            }

            Ok(())
        })
    }


    pub fn get_repository(&self, name: &str) -> Option<&GitRepository> {
        self.repositories.get(name)
    }

    pub fn list_repositories(&self) -> Vec<&GitRepository> {
        self.repositories.values().collect()
    }
}