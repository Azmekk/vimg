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
    pub fn from_cli(cli: &Cli, files: &[PathBuf]) -> Result<Self> {
        let output = if cli.to_folder {
            let first = files
                .first()
                .ok_or_else(|| anyhow!("--to-folder requires at least one input file"))?;
            let folder = unique_batch_folder(first)?;
            fs::create_dir_all(&folder)
                .with_context(|| format!("creating batch folder {}", folder.display()))?;
            Some(folder)
        } else {
            if let Some(dir) = &cli.output
                && !dir.exists()
            {
                fs::create_dir_all(dir)
                    .with_context(|| format!("creating output dir {}", dir.display()))?;
            }
            cli.output.clone()
        };
        if let Some(q) = cli.quality
            && !(1..=100).contains(&q)
        {
            return Err(anyhow!("quality must be between 1 and 100"));
        }
        Ok(Self {
            format: cli.format,
            output,
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
        Format::Jpg | Format::Webp | Format::Avif => {
            let img = convert::decode(path)?;
            convert::encode_to_bytes(&img, format, cfg.quality)?
        }
    };
    let final_bytes = if optimized.len() < bytes.len() {
        optimized
    } else {
        bytes
    };
    let out = resolve_optimize_output(path, cfg)?;
    ensure_parent_exists(&out)?;
    atomic_write(&out, &final_bytes)?;
    Ok(out)
}

fn convert_to(input: &Path, target: Format, cfg: &Config) -> Result<PathBuf> {
    let img = convert::decode(input)?;
    let encoded = convert::encode_to_bytes(&img, target, cfg.quality)?;
    let mut out = convert::resolve_output(input, Some(target), cfg.output.as_deref())?;
    // If -o is set and the candidate output equals the input, fall back to a
    // sibling with a numeric suffix so the original is never clobbered.
    if std::path::absolute(&out).unwrap_or(out.clone())
        == std::path::absolute(input).unwrap_or(input.to_path_buf())
    {
        out = unique_path(&out);
    }
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
            return Ok(unique_path(&candidate));
        }
    }
    Ok(unique_optimized_sibling(input))
}

fn unique_optimized_sibling(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new(""));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let build = |suffix: &str| -> PathBuf {
        let name = if ext.is_empty() {
            format!("{stem}.optimized{suffix}")
        } else {
            format!("{stem}.optimized{suffix}.{ext}")
        };
        parent.join(name)
    };
    let first = build("");
    if !first.exists() {
        return first;
    }
    for n in 1..u32::MAX {
        let candidate = build(&n.to_string());
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or(Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    for n in 1..u32::MAX {
        let name = if ext.is_empty() {
            format!("{stem}{n}")
        } else {
            format!("{stem}{n}.{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

fn unique_batch_folder(first_input: &Path) -> Result<PathBuf> {
    let parent = first_input
        .parent()
        .ok_or_else(|| anyhow!("input {} has no parent", first_input.display()))?;
    let folder_name = parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Images");
    let grandparent = parent.parent().unwrap_or(parent);
    let base = format!("{folder_name}_optimized");
    let first = grandparent.join(&base);
    if !first.exists() {
        return Ok(first);
    }
    for n in 1..u32::MAX {
        let candidate = grandparent.join(format!("{base}{n}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Ok(first)
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
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}
