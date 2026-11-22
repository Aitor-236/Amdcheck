use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "mdcheck",
    version,
    about = "Check local links and images in Markdown files"
)]

pub struct Cli {
    // File or directory to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,
}
