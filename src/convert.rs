use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use image::{DynamicImage, ImageReader};

use crate::cli::Format;

const DEFAULT_JPEG_QUALITY: u8 = 85;
const DEFAULT_WEBP_QUALITY: u8 = 85;
const DEFAULT_AVIF_QUALITY: u8 = 80;
const AVIF_SPEED: u8 = 6;

pub fn resolve_output(
    input: &Path,
    format: Option<Format>,
    output_dir: Option<&Path>,
) -> Result<PathBuf> {
    let mut path = match output_dir {
        Some(dir) => dir.join(
            input
                .file_name()
                .ok_or_else(|| anyhow!("input has no filename: {}", input.display()))?,
        ),
        None => input.to_path_buf(),
    };
    if let Some(fmt) = format {
        path.set_extension(fmt.extension());
    }
    Ok(path)
}

pub fn decode(path: &Path) -> Result<DynamicImage> {
    ImageReader::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("detecting format of {}", path.display()))?
        .decode()
        .with_context(|| format!("decoding {}", path.display()))
}

pub fn encode_to_bytes(img: &DynamicImage, format: Format, quality: Option<u8>) -> Result<Vec<u8>> {
    match format {
        Format::Png => encode_png(img),
        Format::Jpg => encode_jpeg(img, quality),
        Format::Webp => encode_webp(img, quality),
        Format::Avif => encode_avif(img, quality),
    }
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .context("encoding PNG")?;
    let optimized =
        oxipng::optimize_from_memory(&buf, &oxipng::Options::default()).context("oxipng pass")?;
    Ok(optimized)
}

fn encode_jpeg(img: &DynamicImage, quality: Option<u8>) -> Result<Vec<u8>> {
    let q = quality.unwrap_or(DEFAULT_JPEG_QUALITY);
    let rgb = img.to_rgb8();
    let mut buf: Vec<u8> = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, q);
    encoder
        .encode(
            &rgb,
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .context("encoding JPEG")?;
    Ok(buf)
}

fn encode_webp(img: &DynamicImage, quality: Option<u8>) -> Result<Vec<u8>> {
    let q = quality.unwrap_or(DEFAULT_WEBP_QUALITY) as f32;
    let encoder = webp::Encoder::from_image(img).map_err(|e| anyhow!("webp encoder init: {e}"))?;
    Ok(encoder.encode(q).to_vec())
}

fn encode_avif(img: &DynamicImage, quality: Option<u8>) -> Result<Vec<u8>> {
    let q = quality.unwrap_or(DEFAULT_AVIF_QUALITY) as f32;
    let rgba = img.to_rgba8();
    let width = rgba.width() as usize;
    let height = rgba.height() as usize;
    let pixels: Vec<rgb::RGBA8> = rgba
        .pixels()
        .map(|p| rgb::RGBA8 {
            r: p[0],
            g: p[1],
            b: p[2],
            a: p[3],
        })
        .collect();
    let img_ref = ravif::Img::new(pixels.as_slice(), width, height);
    let res = ravif::Encoder::new()
        .with_quality(q)
        .with_speed(AVIF_SPEED)
        .encode_rgba(img_ref)
        .map_err(|e| anyhow!("AVIF encode failed: {e}"))?;
    Ok(res.avif_file)
}
