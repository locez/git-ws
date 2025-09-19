use clap::Parser;
use git_ws::cli::Cli;
use git_ws::workspace::Workspace;
use git_ws::operations::{StatusOperation, AddOperation, CommitOperation};
use git_ws::executor::BatchExecutor;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send>> {
    let cli = Cli::parse();

    // Determine workspace root path
    let workspace_path = if let Some(path) = cli.workspace {
        path
    } else {
        std::env::current_dir().map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?
    };

    // Initialize workspace
    let mut workspace = Workspace::new(workspace_path);
    workspace.discover_repositories().await?;

    // Create batch executor with a concurrency limit
    let executor = BatchExecutor::new(4); // Limit to 4 concurrent operations

    // Execute the requested command
    match &cli.command {
        git_ws::cli::Commands::Status => {
            let operation = Arc::new(StatusOperation);
            let repositories: Vec<_> = workspace.list_repositories().into_iter().cloned().collect();
            let results = executor.execute_operation(operation, repositories).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

            for (repo_name, result) in results {
                match result {
                    Ok(output) => println!("{}", output),
                    Err(e) => eprintln!("Error in {}: {}", repo_name, e),
                }
            }
        }
        git_ws::cli::Commands::Add { paths } => {
            let operation = Arc::new(AddOperation {
                patterns: paths.clone(),
            });
            let repositories: Vec<_> = workspace.list_repositories().into_iter().cloned().collect();
            let results = executor.execute_operation(operation, repositories).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

            for (repo_name, result) in results {
                match result {
                    Ok(output) => println!("{}", output),
                    Err(e) => eprintln!("Error in {}: {}", repo_name, e),
                }
            }
        }
        git_ws::cli::Commands::Commit { message } => {
            let operation = Arc::new(CommitOperation {
                message: message.clone(),
            });
            let repositories: Vec<_> = workspace.list_repositories().into_iter().cloned().collect();
            let results = executor.execute_operation(operation, repositories).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

            for (repo_name, result) in results {
                match result {
                    Ok(output) => println!("{}", output),
                    Err(e) => eprintln!("Error in {}: {}", repo_name, e),
                }
            }
        }
        git_ws::cli::Commands::List => {
            println!("Repositories in workspace:");
            for repo in workspace.list_repositories() {
                println!("  {}", repo.name);
            }
        }
        git_ws::cli::Commands::Exec { command: _ } => {
            // TODO: Implement custom command execution
            println!("Custom command execution is not yet implemented");
        }
    }

    Ok(())
}
