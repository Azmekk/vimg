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
        None => optimize(input, cfg),
        Some(target) => {
            let src_fmt = optimize::detect_format(input).ok();
            if src_fmt == Some(target) {
                eprintln!(
                    "vimg: {}: -f {} matches the source format. Optimization is the right choice here — running that instead. (Drop -f to silence.)",
                    input.display(),
                    target.extension()
                );
                optimize(input, cfg)
            } else {
                convert_to(input, target, cfg)
            }
        }
    }
}

fn optimize(path: &Path, cfg: &Config) -> Result<PathBuf> {
    let format = optimize::detect_format(path)?;
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let optimized = match format {
        Format::Png => optimize::optimize_png_in_memory(&bytes)?,
        // Lossy round-trip for the other formats.
        Format::Jpg | Format::Webp | Format::Avif => {
            let img = convert::decode(path)?;
            convert::encode_to_bytes(&img, format, cfg.quality)?
        }
    };
    if optimized.len() >= bytes.len() {
        eprintln!(
            "vimg: {}: no size improvement ({} -> {} bytes); not writing a copy.",
            path.display(),
            bytes.len(),
            optimized.len()
        );
        return Ok(path.to_path_buf());
    }
    let out = resolve_optimize_output(path, cfg)?;
    ensure_parent_exists(&out)?;
    atomic_write(&out, &optimized)?;
    Ok(out)
}

fn convert_to(input: &Path, target: Format, cfg: &Config) -> Result<PathBuf> {
    let img = convert::decode(input)?;
    let encoded = convert::encode_to_bytes(&img, target, cfg.quality)?;
    let out = convert::resolve_output(input, Some(target), cfg.output.as_deref())?;
    ensure_parent_exists(&out)?;
    atomic_write(&out, &encoded)?;
    Ok(out)
}

fn resolve_optimize_output(input: &Path, cfg: &Config) -> Result<PathBuf> {
    let filename = input
        .file_name()
        .ok_or_else(|| anyhow!("no filename in {}", input.display()))?;
    if let Some(dir) = &cfg.output {
        let candidate = dir.join(filename);
        let candidate_abs = std::path::absolute(&candidate).unwrap_or_else(|_| candidate.clone());
        let input_abs = std::path::absolute(input).unwrap_or_else(|_| input.to_path_buf());
        if candidate_abs != input_abs {
            return Ok(candidate);
        }
    }
    Ok(with_optimized_suffix(input))
}

fn with_optimized_suffix(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new(""));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let new_name = match input.extension().and_then(|e| e.to_str()) {
        Some(ext) if !ext.is_empty() => format!("{stem}.optimized.{ext}"),
        _ => format!("{stem}.optimized"),
    };
    parent.join(new_name)
}

fn ensure_parent_exists(out: &Path) -> Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    Ok(())
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
