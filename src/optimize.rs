use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::cli::Format;

pub fn detect_format(path: &Path) -> Result<Format> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow!("missing or non-UTF-8 extension: {}", path.display()))?
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => Ok(Format::Png),
        "jpg" | "jpeg" => Ok(Format::Jpg),
        "webp" => Ok(Format::Webp),
        "avif" => Ok(Format::Avif),
        other => Err(anyhow!("unsupported in-place format: .{other}")),
    }
}

pub fn optimize_png_in_memory(bytes: &[u8]) -> Result<Vec<u8>> {
    oxipng::optimize_from_memory(bytes, &oxipng::Options::default()).context("oxipng pass")
}
