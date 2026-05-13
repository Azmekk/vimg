use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

const REPO: &str = "Azmekk/vimg";
const USER_AGENT: &str = "vimg-self-updater";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

enum ArchiveKind {
    Zip,
    TarGz,
}

pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("vimg v{current} — checking for updates...");

    let release: Release = ureq::get(&format!(
        "https://api.github.com/repos/{REPO}/releases/latest"
    ))
    .set("User-Agent", USER_AGENT)
    .call()
    .context("querying GitHub releases API")?
    .into_json()
    .context("parsing release JSON")?;

    let latest = release.tag_name.trim_start_matches('v');
    if latest == current {
        println!("Already on the latest version (v{current}).");
        return Ok(());
    }
    println!("Latest: v{latest}");

    let (pattern, kind) = platform_asset()?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains(pattern))
        .ok_or_else(|| anyhow!("no {pattern} asset in release v{latest}"))?;

    println!("Downloading {} ({} bytes)...", asset.name, asset.size);
    let mut archive_bytes = Vec::with_capacity(asset.size.max(1) as usize);
    ureq::get(&asset.browser_download_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .context("downloading release asset")?
        .into_reader()
        .read_to_end(&mut archive_bytes)
        .context("reading downloaded archive")?;

    let new_binary = extract_binary(&archive_bytes, kind)?;
    let staged = stage_new_binary(&new_binary)?;

    self_replace::self_replace(&staged).context("replacing running executable")?;
    let _ = std::fs::remove_file(&staged);

    println!("Updated to v{latest}.");
    Ok(())
}

fn platform_asset() -> Result<(&'static str, ArchiveKind)> {
    if cfg!(windows) {
        Ok(("windows-x86_64", ArchiveKind::Zip))
    } else if cfg!(target_os = "linux") {
        Ok(("linux-x86_64", ArchiveKind::TarGz))
    } else {
        bail!("self-update is only supported on Windows and Linux")
    }
}

fn extract_binary(bytes: &[u8], kind: ArchiveKind) -> Result<Vec<u8>> {
    match kind {
        ArchiveKind::Zip => extract_zip(bytes, "vimg.exe"),
        ArchiveKind::TarGz => extract_targz(bytes, "vimg"),
    }
}

fn extract_zip(bytes: &[u8], name: &str) -> Result<Vec<u8>> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("opening zip archive")?;
    let mut entry = archive
        .by_name(name)
        .with_context(|| format!("{name} not found in zip"))?;
    let mut out = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut out).context("reading zip entry")?;
    Ok(out)
}

fn extract_targz(bytes: &[u8], name: &str) -> Result<Vec<u8>> {
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().context("reading tar entries")? {
        let mut entry = entry.context("reading tar entry")?;
        let path = entry.path().context("entry path")?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if filename == name {
            let mut out = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut out).context("reading tar entry")?;
            return Ok(out);
        }
    }
    bail!("{name} not found in tar archive")
}

fn stage_new_binary(bytes: &[u8]) -> Result<PathBuf> {
    let exe = std::env::current_exe().context("resolving current_exe")?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("current_exe has no parent"))?;
    let filename = if cfg!(windows) {
        "vimg.new.exe"
    } else {
        "vimg.new"
    };
    let staged = dir.join(filename);

    let mut f =
        std::fs::File::create(&staged).with_context(|| format!("creating {}", staged.display()))?;
    f.write_all(bytes)
        .with_context(|| format!("writing {}", staged.display()))?;
    f.sync_all().ok();
    drop(f);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .context("chmod 755 on staged binary")?;
    }

    Ok(staged)
}
