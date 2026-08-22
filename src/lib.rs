use std::sync::OnceLock;
use std::time::Instant;

use image::{
    Delay, DynamicImage, Frame, ImageFormat,
    codecs::gif::{GifEncoder, Repeat},
};
use rayon::prelude::*;

const FRAMES_BYTES: [&'static [u8]; 10] = [
    include_bytes!("../assets/pet0.gif"),
    include_bytes!("../assets/pet1.gif"),
    include_bytes!("../assets/pet2.gif"),
    include_bytes!("../assets/pet3.gif"),
    include_bytes!("../assets/pet4.gif"),
    include_bytes!("../assets/pet5.gif"),
    include_bytes!("../assets/pet6.gif"),
    include_bytes!("../assets/pet7.gif"),
    include_bytes!("../assets/pet8.gif"),
    include_bytes!("../assets/pet9.gif"),
];

/// Options controlling the generated petpet GIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetpetOptions {
    /// Width and height of the output GIF in pixels (aspect ratio is always 1:1).
    pub resolution: u32,
    /// Whether the input image should be masked into a circle.
    pub rounded: bool,
}

impl Default for PetpetOptions {
    fn default() -> Self {
        Self {
            resolution: 128,
            rounded: false,
        }
    }
}

static PRELOADED_FRAMES: OnceLock<Vec<image::RgbaImage>> = OnceLock::new();

fn get_frames() -> &'static [image::RgbaImage] {
    PRELOADED_FRAMES.get_or_init(|| {
        FRAMES_BYTES
            .iter()
            .map(|bytes| {
                image::load_from_memory_with_format(bytes, ImageFormat::Gif)
                    .expect("Failed to load asset frame")
                    .to_rgba8()
            })
            .collect()
    })
}

/// Masks the image into a circle centered in the frame, with radius equal to
/// half the smaller dimension. Edge pixels get ~1px of antialiasing.
fn apply_circle_mask(image: &mut image::RgbaImage) {
    let (width, height) = image.dimensions();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = width.min(height) as f32 / 2.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let dist = (dx * dx + dy * dy).sqrt();
            let coverage = (radius - dist + 0.5).clamp(0.0, 1.0);
            if coverage < 1.0 {
                let pixel = image.get_pixel_mut(x, y);
                pixel[3] = (pixel[3] as f32 * coverage).round() as u8;
            }
        }
    }
}

pub fn petpet(input: &DynamicImage, options: PetpetOptions) -> Vec<u8> {
    let PetpetOptions {
        resolution,
        rounded,
    } = options;
    assert!(resolution > 0, "resolution must be non-zero");

    let start_time = Instant::now();
    tracing::info!("Starting petpet processing...");

    let mut resized_input_rgba = input
        .resize_exact(
            resolution,
            resolution,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();

    if rounded {
        apply_circle_mask(&mut resized_input_rgba);
    }

    let frames = get_frames();

    tracing::info!("Generating {} frames in parallel...", frames.len());
    let gen_start = Instant::now();
    let processed_frames: Vec<Frame> = (0..frames.len())
        .into_par_iter()
        .map(|index| {
            let frame_image = image::imageops::resize(
                &frames[index],
                resolution,
                resolution,
                image::imageops::FilterType::Triangle,
            );
            let mut base = image::RgbaImage::new(resolution, resolution);

            let j = if index < frames.len() / 2 {
                index
            } else {
                frames.len() - index
            };

            let scale_x = 0.8 + j as f32 * 0.02;
            let scale_y = 0.8 - j as f32 * 0.05;
            let offset_x = (1.0 - scale_x) * 0.5 + 0.1;
            let offset_y = 1.0 - scale_y - 0.08;

            let warped_input = image::imageops::resize(
                &resized_input_rgba,
                (resolution as f32 * scale_x) as u32,
                (resolution as f32 * scale_y) as u32,
                image::imageops::FilterType::Nearest,
            );

            image::imageops::overlay(
                &mut base,
                &warped_input,
                (resolution as f32 * offset_x) as i64,
                (resolution as f32 * offset_y) as i64,
            );

            image::imageops::overlay(&mut base, &frame_image, 0, 0);

            let delay = Delay::from_numer_denom_ms(30, 1);
            Frame::from_parts(base, 0, 0, delay)
        })
        .collect();
    let gen_duration = gen_start.elapsed();
    tracing::info!("Frame generation took: {:?}", gen_duration);

    tracing::info!("Encoding GIF...");
    let enc_start = Instant::now();
    let mut buffer = Vec::new();
    let mut encoder = GifEncoder::new(&mut buffer);
    encoder.set_repeat(Repeat::Infinite).unwrap();

    encoder.encode_frames(processed_frames).unwrap();

    drop(encoder);
    let enc_duration = enc_start.elapsed();
    tracing::info!("GIF encoding took: {:?}", enc_duration);
    tracing::info!("Total petpet time: {:?}", start_time.elapsed());

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_resolution_produces_matching_gif_size() {
        let input = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            64,
            image::Rgba([255, 0, 0, 255]),
        ));
        let output = petpet(
            &input,
            PetpetOptions {
                resolution: 64,
                rounded: true,
            },
        );

        // GIF logical screen width/height live at bytes 6..10, little-endian.
        assert_eq!(u16::from_le_bytes([output[6], output[7]]), 64);
        assert_eq!(u16::from_le_bytes([output[8], output[9]]), 64);
    }

    #[test]
    fn circle_mask_keeps_center_and_clears_corners() {
        let mut img = image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]));
        apply_circle_mask(&mut img);

        // Outside the circle.
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(63, 63)[3], 0);
        assert_eq!(img.get_pixel(63, 0)[3], 0);
        // Inside the circle.
        assert_eq!(img.get_pixel(32, 32)[3], 255);
        assert_eq!(img.get_pixel(32, 1)[3], 255);
        assert_eq!(img.get_pixel(5, 32)[3], 255);
    }
}
