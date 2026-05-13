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

## Usage

```bash
vimg photo.png                    # optimize in place (lossless for PNG)
vimg *.png                        # optimize a batch in parallel
vimg photo.png -f webp            # convert; writes photo.webp next to the original
vimg photo.png -f avif -q 75      # convert at a specific quality
vimg *.jpg -f webp -o ./out       # batch convert into ./out
```

| Flag | Meaning |
|---|---|
| `-f, --format` | Target format: `png`, `jpg`, `webp`, `avif` |
| `-q, --quality` | Quality 1–100 (lossy formats only) |
| `-o, --output` | Output directory (defaults to alongside the input) |

Without `-f`, vimg optimizes the file in place. With `-f`, the original is preserved and a new file is written with the target extension.

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
