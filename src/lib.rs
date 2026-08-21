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

static PRELOADED_FRAMES: OnceLock<Vec<image::RgbaImage>> = OnceLock::new();

fn get_frames() -> &'static [image::RgbaImage] {
    PRELOADED_FRAMES.get_or_init(|| {
        FRAMES_BYTES
            .iter()
            .map(|bytes| {
                let img = image::load_from_memory_with_format(bytes, ImageFormat::Gif)
                    .expect("Failed to load asset frame");
                img.resize_exact(128, 128, image::imageops::FilterType::Triangle)
                    .to_rgba8()
            })
            .collect()
    })
}

pub fn petpet(input: &DynamicImage) -> Vec<u8> {
    let start_time = Instant::now();
    tracing::info!("Starting petpet processing...");

    let resized_input = input.resize_exact(128, 128, image::imageops::FilterType::Triangle);
    let resized_input_rgba = resized_input.to_rgba8();
    let frames = get_frames();

    tracing::info!("Generating {} frames in parallel...", frames.len());
    let gen_start = Instant::now();
    let processed_frames: Vec<Frame> = (0..frames.len())
        .into_par_iter()
        .map(|index| {
            let frame_image = &frames[index];
            let mut base = image::RgbaImage::new(128, 128);

            let j = if index < frames.len() / 2 {
                index
            } else {
                frames.len() - index
            };

            let width = 0.8 + j as f32 * 0.02;
            let height = 0.8 - j as f32 * 0.05;
            let offset_x = (1.0 - width) * 0.5 + 0.1;
            let offset_y = 1.0 - height - 0.08;

            let warped_input = image::imageops::resize(
                &resized_input_rgba,
                (128.0 * width) as u32,
                (128.0 * height) as u32,
                image::imageops::FilterType::Nearest,
            );

            image::imageops::overlay(
                &mut base,
                &warped_input,
                (128.0 * offset_x) as i64,
                (128.0 * offset_y) as i64,
            );

            image::imageops::overlay(&mut base, frame_image, 0, 0);

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
