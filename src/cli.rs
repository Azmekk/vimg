use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "vimg",
    version,
    about = "Local image optimization and conversion CLI"
)]
pub struct Cli {
    /// Input files to process.
    pub files: Vec<PathBuf>,

    /// Target format. If omitted, optimize in place.
    #[arg(short, long, value_enum)]
    pub format: Option<Format>,

    /// Output directory. Defaults to alongside each input file.
    #[arg(short, long, conflicts_with = "to_folder")]
    pub output: Option<PathBuf>,

    /// Place outputs into a sibling "<source-folder>_optimized" directory.
    /// Useful when batch-converting multiple files. Conflicts with -o.
    #[arg(long)]
    pub to_folder: bool,

    /// Quality (1-100). Format-specific defaults apply when omitted.
    #[arg(short, long)]
    pub quality: Option<u8>,

    /// Install the Windows Explorer context menu entries (Windows only).
    #[arg(long, conflicts_with = "disable_context_menu")]
    pub enable_context_menu: bool,

    /// Remove the Windows Explorer context menu entries (Windows only).
    #[arg(long)]
    pub disable_context_menu: bool,

    /// Replace this binary with the latest GitHub release.
    #[arg(long)]
    pub update: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpg,
    Webp,
    Avif,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::Webp => "webp",
            Format::Avif => "avif",
        }
    }
}
