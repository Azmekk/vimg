# vimg

A fast, local image converter and optimizer. Run from the CLI or right-click on Windows.

Supports **PNG**, **JPEG**, **WebP**, and **AVIF**. Reads GIF / BMP / TIFF too.

## Install

**Windows:**

```powershell
irm https://raw.githubusercontent.com/Azmekk/vimg/master/install.ps1 | iex
```

**Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/Azmekk/vimg/master/install.sh | bash
```

Both scripts grab the latest release from GitHub and drop `vimg` on your PATH.

To update later, just run:

```
vimg --update
```

vimg fetches the latest release and replaces its own binary in place. No need to re-run the install script.

## Usage

```bash
vimg photo.png                    # writes photo.optimized.png next to the original
vimg *.png                        # optimize a batch in parallel
vimg photo.png -f webp            # convert; writes photo.webp next to the original
vimg photo.png -f avif -q 75      # convert at a specific quality
vimg *.jpg -f webp -o ./out       # batch convert into ./out
vimg *.jpg -f webp --to-folder    # batch convert into <folder>_optimized/
vimg **/*.png                     # recurse into subfolders
```

Outputs never overwrite existing files: vimg appends numeric suffixes (`name.optimized.png`, `name.optimized1.png`, `name.optimized2.png`, …) and `--to-folder` creates `<folder>_optimized`, `<folder>_optimized1`, `<folder>_optimized2`, … as needed.

Without `-f`, vimg **optimizes the file and writes a sibling copy** (`name.optimized.ext`) — the original is never touched. With `-o <dir>` the copy goes into that directory with the original filename. If `-f <ext>` matches the source extension, vimg prints a notice and falls through to the optimize path automatically. If the optimized result isn't smaller than the input, no copy is written.

With `-f` (a different format), the original is preserved and a new file is written with the target extension.

Glob patterns (`*`, `?`, `[...]`, `**`) are expanded by vimg itself, so they work the same on Windows PowerShell as they do in bash. On Windows, matching is case-insensitive — `*.jpg` matches `.JPG` too.

| Flag | Meaning |
|---|---|
| `-f, --format` | Target format: `png`, `jpg`, `webp`, `avif` |
| `-q, --quality` | Quality 1–100 (lossy formats only) |
| `-o, --output` | Output directory (defaults to alongside the input) |

Per-format defaults: PNG runs through `oxipng`; JPEG and WebP use quality 85; AVIF uses quality 80 at `ravif` speed 6.

## Windows Explorer integration

Install once:

```powershell
vimg --enable-context-menu
```

Right-click any image to get a **"Convert with vimg"** submenu with the four target formats plus an **Optimize** entry. On Windows 11 the items live under *Show more options* (shift-right-click).

Uninstall with `vimg --disable-context-menu`. Registry edits are scoped to `HKCU` — no admin required.

## Build from source

```bash
cargo build --release
```

Requires a recent Rust toolchain (edition 2024). On Linux you'll need `libclang-dev` for the `webp` crate's bindgen step.

## License

MIT.
