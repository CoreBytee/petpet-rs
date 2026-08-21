use image::ImageFormat;

fn main() {
    println!("Hello, world!");

    let test_data = include_bytes!("../assets/sample.png");
    let test_image = image::load_from_memory_with_format(test_data, ImageFormat::Png).unwrap();

    let output = petpet_rs::petpet(&test_image);
    std::fs::write("output.gif", output).unwrap();
}
