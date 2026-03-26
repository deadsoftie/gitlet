mod cli;
mod config;
mod exclude;
mod gitlet;
mod repo;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let git_root = repo::find_git_root()?;
            let name = name.as_deref().unwrap_or("default");
            gitlet::init(&git_root, name)
        }
        Commands::Add { .. } => {
            println!("[add] not yet implemented");
            Ok(())
        }
        Commands::Remove { .. } => {
            println!("[remove] not yet implemented");
            Ok(())
        }
        Commands::Commit { .. } => {
            println!("[commit] not yet implemented");
            Ok(())
        }
        Commands::Status { .. } => {
            println!("[status] not yet implemented");
            Ok(())
        }
        Commands::Log { .. } => {
            println!("[log] not yet implemented");
            Ok(())
        }
        Commands::List => {
            println!("[list] not yet implemented");
            Ok(())
        }
        Commands::Switch { .. } => {
            println!("[switch] not yet implemented");
            Ok(())
        }
    }
}
