mod cli;
mod repo;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<()> = match cli.command {
        Commands::Init { .. } => {
            println!("[init] not yet implemented");
            Ok(())
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
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
