mod cli;
mod context_menu;
mod convert;
mod optimize;
mod pipeline;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.enable_context_menu {
        return context_menu::enable();
    }
    if cli.disable_context_menu {
        return context_menu::disable();
    }

    if cli.files.is_empty() {
        eprintln!("vimg: no input files. Run `vimg --help` for usage.");
        std::process::exit(2);
    }

    let cfg = pipeline::Config::from_cli(&cli)?;
    let failures = pipeline::run(&cli.files, &cfg)?;

    if !failures.is_empty() {
        for (path, err) in &failures {
            eprintln!("vimg: {}: {err:#}", path.display());
        }
        std::process::exit(1);
    }
    Ok(())
}
