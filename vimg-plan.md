# vimg — Project Plan

A local, high-performance image optimization CLI written in Rust. Converts between PNG, JPEG, WebP, and AVIF and optimizes in place. Single-line install on Windows and Linux, optional Windows Explorer context menu, and tagged releases built via GitHub Actions.

## 1. Project layout

```
vimg/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── convert.rs
│   ├── optimize.rs
│   ├── pipeline.rs
│   └── context_menu.rs        # Windows only via #[cfg(windows)]
├── install.ps1
├── install.sh
├── .github/workflows/
│   ├── ci.yml
│   └── release.yml
└── README.md
```

## 2. Cargo.toml

```toml
[package]
name = "vimg"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
image = "0.25"
ravif = "0.11"
rgb = "0.8"
webp = "0.3"
oxipng = "9"
rayon = "1.10"
anyhow = "1"
thiserror = "1"
indicatif = "0.17"

[target.'cfg(windows)'.dependencies]
windows-registry = "0.2"

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = 3
```

## 3. CLI surface

Flat. No subcommands for the core workflow. The verbs are inferred from flags.

```
vimg <files>...                        # optimize each in place
vimg <files>... -f <fmt>               # convert each (new file with target extension)
vimg <files>... -f <fmt> -q 90         # convert at a specific quality
vimg --enable-context-menu             # Windows only; no-op elsewhere with a message
vimg --disable-context-menu
```

Default output rules:

- No `-f`: overwrites the input file (in-place optimization, matching `oxipng`/`jpegoptim` convention).
- With `-f`: writes alongside the input with the new extension (`photo.png` → `photo.webp`); original is preserved.
- `-o <dir>`: writes all outputs into that directory.

clap derive sketch:

```rust
#[derive(Parser)]
#[command(name = "vimg", version)]
struct Cli {
    files: Vec<PathBuf>,

    #[arg(short, long)]
    format: Option<Format>,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long)]
    quality: Option<u8>,

    #[arg(long, conflicts_with = "disable_context_menu")]
    enable_context_menu: bool,

    #[arg(long)]
    disable_context_menu: bool,
}

#[derive(ValueEnum, Clone, Copy)]
enum Format { Png, Jpg, Webp, Avif }
```

Per-format defaults:

- **PNG** — `oxipng` default preset (lossless).
- **JPEG** — quality 85.
- **WebP** — quality 85.
- **AVIF** — `ravif` quality 80 at speed 6.

`-q`/`--quality` (1–100) overrides the default for lossy formats.

Batch processing wraps the file list in `rayon`'s `par_iter`, with `indicatif::MultiProgress` showing one bar per file.

## 4. Context menu (Windows only)

Registered under `HKEY_CURRENT_USER` — no admin needed. Layout uses `ExtendedSubCommandsKey` so the submenu structure lives in one shared location instead of being duplicated under every image extension. Each extension only needs a small pointer entry.

### Menu structure

```
Convert with vimg
├── Convert to .png
├── Convert to .jpg
├── Convert to .webp
├── Convert to .avif
├── ──────────────
└── Optimize
```

### Registry layout

```
HKCU\Software\Classes\SystemFileAssociations\<.ext>\shell\vimg
    MUIVerb                = "Convert with vimg"
    Icon                   = "<install-dir>\vimg.exe,0"
    ExtendedSubCommandsKey = "vimg.Menu"        ; HKCR-relative ProgID name

HKCU\Software\Classes\vimg.Menu\shell
    \01-png\command   →  "<exe>" "%1" -f png
    \02-jpg\command   →  "<exe>" "%1" -f jpg
    \03-webp\command  →  "<exe>" "%1" -f webp
    \04-avif\command  →  "<exe>" "%1" -f avif
    \05-opt
        MUIVerb        = "Optimize"
        CommandFlags   = 0x40                      ; preceding separator
    \05-opt\command   →  "<exe>" "%1"
```

Register the extension pointer under each of `.png`, `.jpg`, `.jpeg`, `.webp`, `.avif`, `.gif`, `.bmp`, `.tif`, `.tiff`. The shared `vimg.Menu` key is written once.

`--disable-context-menu` deletes the `shell\vimg` subkey under each extension and removes the shared menu key. Clean uninstall.

**Windows 11 caveat:** the entry appears in the legacy menu (shift-right-click, or "Show more options"). The modern menu requires a packaged MSIX with a signed `IExplorerCommand` COM handler — significantly more work and worth deferring until the tool earns it.

### Non-Windows behavior

`--enable-context-menu` and `--disable-context-menu` print a message explaining the feature is Windows-only and exit successfully. No fallback. Linux is usable via the CLI only.

## 5. Install scripts

### install.ps1

```powershell
$ErrorActionPreference = 'Stop'

$repo = 'Azmekk/vimg'
$installDir = Join-Path $env:LOCALAPPDATA 'vimg'

$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$asset = $release.assets | Where-Object { $_.name -like '*windows-x86_64*.zip' } | Select-Object -First 1
if (-not $asset) { throw 'No Windows asset found in latest release.' }

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
$zip = Join-Path $installDir 'vimg.zip'
Invoke-WebRequest $asset.browser_download_url -OutFile $zip
Expand-Archive -Force $zip $installDir
Remove-Item $zip

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
}

Write-Host "vimg installed to $installDir"
Write-Host 'Restart your terminal, then optionally run: vimg --enable-context-menu'
```

Invocation:

```powershell
irm https://raw.githubusercontent.com/Azmekk/vimg/master/install.ps1 | iex
```

### install.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="Azmekk/vimg"
INSTALL_DIR="${HOME}/.local/bin"

case "$(uname -s)" in
    Linux*)  OS=linux ;;
    *) echo "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

PATTERN="vimg-${OS}-x86_64"
URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep "browser_download_url.*${PATTERN}" \
    | head -1 \
    | cut -d '"' -f 4)

[ -z "${URL}" ] && { echo "No release asset for ${OS}"; exit 1; }

mkdir -p "${INSTALL_DIR}"
TMP=$(mktemp -d)
trap 'rm -rf "${TMP}"' EXIT

curl -fsSL "${URL}" -o "${TMP}/vimg.tar.gz"
tar -xzf "${TMP}/vimg.tar.gz" -C "${TMP}"
install -m 755 "${TMP}/vimg" "${INSTALL_DIR}/vimg"

echo "vimg installed to ${INSTALL_DIR}/vimg"
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "Note: add ${INSTALL_DIR} to your PATH" ;;
esac
```

Invocation:

```bash
curl -fsSL https://raw.githubusercontent.com/Azmekk/vimg/master/install.sh | bash
```

## 6. GitHub Actions

Two workflows: CI on every push (fmt, clippy, test) and release on tag push (build matrix + upload).

### .github/workflows/release.yml

```yaml
name: release
on:
  push:
    tags: ['v*']

permissions:
  contents: write

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: vimg-windows-x86_64
            archive: zip
          - os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
            artifact: vimg-linux-x86_64
            archive: tar.gz
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}

      - name: Package (Unix)
        if: matrix.archive == 'tar.gz'
        run: |
          mkdir dist
          cp target/${{ matrix.target }}/release/vimg dist/
          tar -czf ${{ matrix.artifact }}.tar.gz -C dist .

      - name: Package (Windows)
        if: matrix.archive == 'zip'
        run: |
          mkdir dist
          cp target/${{ matrix.target }}/release/vimg.exe dist/
          Compress-Archive -Path dist/* -DestinationPath ${{ matrix.artifact }}.zip

      - uses: softprops/action-gh-release@v2
        with:
          files: |
            ${{ matrix.artifact }}.tar.gz
            ${{ matrix.artifact }}.zip
          fail_on_unmatched_files: false
```

Notes on the runner choices:

- **`ubuntu-22.04`** rather than `ubuntu-latest` for glibc compatibility — newer Ubuntu links against a glibc that older distros don't have. For full libc independence, switch the target to `x86_64-unknown-linux-musl` and install the musl toolchain.

## 7. Build order

A path that gets to a usable tool fast and a shippable release shortly after:

1. `cargo new vimg`, wire up the flat clap CLI with stub handlers.
2. Bare `vimg <files>` working for PNG optimization via `oxipng`.
3. Add encode/decode for JPEG, WebP, AVIF via `image` + `webp` + `ravif`.
4. `-f` conversion path with per-format defaults and `-q` override.
5. `rayon` parallel batch with `indicatif` progress.
6. `context_menu.rs` (Windows) and `--enable/--disable-context-menu` handlers.
7. `install.ps1` + `install.sh`.
8. Release workflow, tag `v0.1.0`, verify both install scripts and the context menu install end-to-end.

Usable tool by step 5, shippable v0.1 by step 8.
