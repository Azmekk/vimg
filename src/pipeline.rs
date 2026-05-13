use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::cli::{Cli, Format};
use crate::convert;
use crate::optimize;

pub struct Config {
    pub format: Option<Format>,
    pub output: Option<PathBuf>,
    pub quality: Option<u8>,
}

impl Config {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        if let Some(dir) = &cli.output
            && !dir.exists()
        {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating output dir {}", dir.display()))?;
        }
        if let Some(q) = cli.quality
            && !(1..=100).contains(&q)
        {
            return Err(anyhow!("quality must be between 1 and 100"));
        }
        Ok(Self {
            format: cli.format,
            output: cli.output.clone(),
            quality: cli.quality,
        })
    }
}

pub fn run(files: &[PathBuf], cfg: &Config) -> Result<Vec<(PathBuf, anyhow::Error)>> {
    let multi = MultiProgress::new();
    let style = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());

    let failures: Vec<(PathBuf, anyhow::Error)> = files
        .par_iter()
        .filter_map(|path| {
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(style.clone());
            bar.set_message(format!("{}", path.display()));
            bar.enable_steady_tick(std::time::Duration::from_millis(120));

            let result = process_one(path, cfg);
            match &result {
                Ok(out) => bar.finish_with_message(format!("✓ {}", out.display())),
                Err(e) => bar.finish_with_message(format!("✗ {} — {e:#}", path.display())),
            }
            result.err().map(|e| (path.clone(), e))
        })
        .collect();

    Ok(failures)
}

fn process_one(input: &Path, cfg: &Config) -> Result<PathBuf> {
    match cfg.format {
        None => optimize_in_place(input),
        Some(target) => convert_to(input, target, cfg),
    }
}

fn optimize_in_place(path: &Path) -> Result<PathBuf> {
    let format = optimize::detect_format(path)?;
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let optimized = match format {
        Format::Png => optimize::optimize_png_in_memory(&bytes)?,
        // Other formats: re-encode through the decoder. Lossy.
        Format::Jpg | Format::Webp | Format::Avif => {
            let img = convert::decode(path)?;
            convert::encode_to_bytes(&img, format, None)?
        }
    };
    if optimized.len() < bytes.len() {
        atomic_write(path, &optimized)?;
    }
    Ok(path.to_path_buf())
}

fn convert_to(input: &Path, target: Format, cfg: &Config) -> Result<PathBuf> {
    let img = convert::decode(input)?;
    let encoded = convert::encode_to_bytes(&img, target, cfg.quality)?;
    let out = convert::resolve_output(input, Some(target), cfg.output.as_deref())?;
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    atomic_write(&out, &encoded)?;
    Ok(out)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!(
        "{}.vimg.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        f.write_all(bytes)
            .with_context(|| format!("writing {}", tmp.display()))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}
