use std::path::PathBuf;
use clap::Parser;
use mcp_rs::filesystem_server::FileSystemServer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directories to allow access to
    #[arg(required = true)]
    allowed_directories: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Validate directories
    for dir in &cli.allowed_directories {
        if !dir.is_dir() {
            eprintln!("Error: {} is not a directory", dir.display());
            std::process::exit(1);
        }
    }

    // Create and run server
    let mut server = FileSystemServer::new(cli.allowed_directories);
    server.run().await?;

    Ok(())
}
