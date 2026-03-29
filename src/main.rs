mod cli;
mod config;
mod exclude;
mod gitnook;
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
            gitnook::init(&git_root, name)
        }
        Commands::Add { files, to } => {
            let git_root = repo::find_git_root()?;
            gitnook::add(&git_root, &files, to.as_deref())
        }
        Commands::Remove { file, to } => {
            let git_root = repo::find_git_root()?;
            gitnook::remove(&git_root, &file, to.as_deref())
        }
        Commands::Commit { m, to } => {
            let git_root = repo::find_git_root()?;
            gitnook::commit(&git_root, &m, to.as_deref())
        }
        Commands::Status { name } => {
            let git_root = repo::find_git_root()?;
            gitnook::status(&git_root, name.as_deref())
        }
        Commands::Log { name } => {
            let git_root = repo::find_git_root()?;
            gitnook::log(&git_root, name.as_deref())
        }
        Commands::List => {
            let git_root = repo::find_git_root()?;
            gitnook::list(&git_root)
        }
        Commands::Switch { name } => {
            let git_root = repo::find_git_root()?;
            gitnook::switch(&git_root, &name)
        }
        Commands::Diff { name } => {
            let git_root = repo::find_git_root()?;
            gitnook::diff(&git_root, name.as_deref())
        }
        Commands::Destroy { name } => {
            let git_root = repo::find_git_root()?;
            gitnook::destroy(&git_root, &name)
        }
    }
}
