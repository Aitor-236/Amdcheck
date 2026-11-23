mod checker;
mod cli;
mod markdown;
mod reporter;
mod scanner;
use clap::Parser;
use std::io;

fn main() -> std::io::Result<()> {
    let cli = cli::Cli::parse();
    let mut results = Vec::new();

    let files = scanner::collect_md_files(&cli.path)?;
    let root_dir = cli.path.canonicalize().ok();

    for file in files {
        let content = std::fs::read_to_string(&file)?;
        let links = markdown::extract_links(&content);
        let checked = checker::check_links(&file, &links, root_dir.as_deref());
        results.extend(checked);
    }

    let stdout = io::stdout();
    let mut output = stdout.lock();
    reporter::print_report(&mut output, &results)?;

    if reporter::has_broken_links(&results) {
        std::process::exit(1);
    }

    Ok(())
}
