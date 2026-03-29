use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitnook", about = "Lightweight local git contexts inside a repo", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new named gitnook
    Init {
        /// Name of the gitnook (default: "default")
        name: Option<String>,
    },
    /// Stage one or more files in a gitnook
    Add {
        /// Files to stage
        #[arg(required = true)]
        files: Vec<String>,
        /// Target gitnook (overrides active)
        #[arg(long)]
        to: Option<String>,
    },
    /// Untrack a file from a gitnook
    Remove {
        /// File to untrack
        file: String,
        /// Target gitnook (overrides active)
        #[arg(long)]
        to: Option<String>,
    },
    /// Commit staged changes in a gitnook
    Commit {
        /// Commit message
        #[arg(short)]
        m: String,
        /// Target gitnook (overrides active)
        #[arg(long)]
        to: Option<String>,
    },
    /// Show status of gitnooks
    Status {
        /// Name of a specific gitnook
        name: Option<String>,
    },
    /// Show commit history of a gitnook
    Log {
        /// Name of a specific gitnook
        name: Option<String>,
    },
    /// List all gitnooks in the repo
    List,
    /// Switch the active gitnook
    Switch {
        /// Name of the gitnook to activate
        name: String,
    },
    /// Show working-tree diff against the last gitnook commit
    Diff {
        /// Name of a specific gitnook (defaults to active)
        name: Option<String>,
    },
    /// Permanently delete a gitnook and clean up all its exclusions
    Destroy {
        /// Name of the gitnook to destroy
        name: String,
    },
}
