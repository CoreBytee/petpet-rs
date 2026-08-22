use image::ImageFormat;
use petpet_rs::PetpetOptions;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Hello, world!");

    let test_data = include_bytes!("../assets/sample2.png");
    let test_image = image::load_from_memory_with_format(test_data, ImageFormat::Png).unwrap();

    let options = PetpetOptions {
        resolution: 512,
        rounded: true,
        quality: 10,
    };
    let output = petpet_rs::petpet(&test_image, options);
    std::fs::write("output.gif", output).unwrap();
}
