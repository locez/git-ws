use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "git-ws", version = "0.1.0", author = "Locez <loki.a@live.cn>")]
#[clap(about = "A git workspace manager tool", long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    /// Set the workspace root path
    #[clap(short, long, value_parser, value_name = "PATH")]
    pub workspace: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show the status of all repositories in the workspace
    Status,

    /// Add files to the index of all repositories
    Add {
        /// Files to add
        #[clap(required = true, value_parser)]
        paths: Vec<String>,
    },

    /// Commit changes in all repositories
    Commit {
        /// Commit message
        #[clap(short, long, value_parser)]
        message: String,
    },

    /// List all repositories in the workspace
    List,

    /// Execute a custom git command in all repositories
    Exec {
        /// The command to execute
        #[clap(required = true, value_parser)]
        command: Vec<String>,
    },
}