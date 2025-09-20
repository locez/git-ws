use clap::Parser;
use git_ws::cli::Cli;
use git_ws::executor::BatchExecutor;
use git_ws::operations::{AddOperation, CommitOperation, FileStatus, StatusOperation};
use git_ws::workspace::Workspace;
use serde_json;
use std::sync::Arc;
use tabled::Table;
use tabled::settings::object::Segment;
use tabled::settings::{Alignment, Merge, Modify, Style};

// Custom function to display status in the desired table format
fn display_status_table(repo_statuses: &[(String, Vec<FileStatus>)]) {
    if repo_statuses.is_empty() {
        return;
    }

    // Calculate summary counts for each repository
    let mut repo_summaries: Vec<(String, usize, usize)> = Vec::new(); // (repo_name, untracked_count, modified_count)

    for (repo_name, statuses) in repo_statuses {
        let mut untracked_count = 0;
        let mut modified_count = 0;

        for status in statuses {
            if status.status.contains("Untracked") {
                untracked_count += 1;
            } else if status.status.contains("Modified") {
                modified_count += 1;
            }
        }

        repo_summaries.push((repo_name.clone(), untracked_count, modified_count));
    }

    // Create a vector to hold all the table rows
    let mut table_rows: Vec<FileStatus> = Vec::new();

    // Process each repository
    for (idx, (repo_name, statuses)) in repo_statuses.iter().enumerate() {
        let summary_info = repo_summaries
            .iter()
            .find(|(name, _, _)| name == repo_name)
            .unwrap();
        let untracked_count = summary_info.1;
        let modified_count = summary_info.2;

        // Add each file status with appropriate repository and summary info
        for (file_idx, status) in statuses.iter().enumerate() {
            let mut row = status.clone();

            // For the first file of each repository, show the repository name and summary
            if file_idx == 0 {
                row.repository = repo_name.clone();
                row.summary = format!(
                    "Untracked: {}\nModified: {}",
                    untracked_count, modified_count
                );
            } else {
                // For subsequent files, leave repository and summary empty
                row.repository = String::new();
                row.summary = String::new();
            }

            table_rows.push(row.clone());
        }

        // Add a separator row after each repository except the last one
        // if idx < repo_statuses.len() - 1 {
        //     table_rows.push(FileStatus {
        //         repository: String::new(),
        //         summary: String::new(),
        //         status: String::new(),
        //         file: String::new(),
        //     });
        // }
    }

    // Create and display the table
    let mut table = Table::new(&table_rows);
    table
        .with(Style::modern())
        .with(Merge::vertical())
        .with(Modify::new(Segment::all()).with(Alignment::center()));

    println!("{}", table);
}

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
            let results = executor
                .execute_operation(operation, repositories)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

            // Collect file statuses grouped by repository
            let mut repo_statuses: Vec<(String, Vec<FileStatus>)> = Vec::new();
            let mut has_errors = false;

            for (repo_name, result) in results {
                match result {
                    Ok(output) => {
                        // Parse the JSON output to extract FileStatus objects
                        match serde_json::from_str::<Vec<FileStatus>>(&output) {
                            Ok(statuses) => repo_statuses.push((repo_name, statuses)),
                            Err(e) => {
                                eprintln!("Error parsing status for {}: {}", repo_name, e);
                                has_errors = true;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error in {}: {}", repo_name, e);
                        has_errors = true;
                    }
                }
            }

            // Create a custom table with the desired format
            display_status_table(&repo_statuses);

            if has_errors {
                std::process::exit(1);
            }
        }
        git_ws::cli::Commands::Add { paths } => {
            let operation = Arc::new(AddOperation {
                patterns: paths.clone(),
            });
            let repositories: Vec<_> = workspace.list_repositories().into_iter().cloned().collect();
            let results = executor
                .execute_operation(operation, repositories)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

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
            let results = executor
                .execute_operation(operation, repositories)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

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
