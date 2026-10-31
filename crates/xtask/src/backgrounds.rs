//! Ballroom strips and parallax contact sheets, using the runtime's seed and palette.

use assetgen::backgrounds::{self, DEFAULT_LIGHT, DEFAULT_SEED, Layer};
use image::{
    Rgba, RgbaImage,
    imageops::{FilterType, resize},
};

fn preview(layers: &[Layer], camera: f32) -> RgbaImage {
    let mut out = RgbaImage::new(640, 360);
    for layer in layers {
        let offset = backgrounds::scroll(camera, layer.parallax);
        for (x, y, pixel) in out.enumerate_pixels_mut() {
            let sx = (x as i32 + offset).rem_euclid(backgrounds::WIDTH as i32) as u32;
            let source = layer.image.get_pixel(sx, y + 20);
            if source[3] != 0 {
                *pixel = *source;
            }
        }
    }
    out
}

pub fn render() -> crate::Result<()> {
    let out = crate::root().join("target/backgrounds");
    std::fs::create_dir_all(&out)?;
    let start = std::time::Instant::now();
    let layers = backgrounds::ballroom(DEFAULT_SEED, DEFAULT_LIGHT);
    println!("Ballroom generated in {:?}", start.elapsed());
    for layer in &layers {
        layer.image.save(out.join(format!("{}.png", layer.name)))?;
        resize(
            &layer.image,
            backgrounds::WIDTH * 2,
            backgrounds::HEIGHT * 2,
            FilterType::Nearest,
        )
        .save(out.join(format!("{}-2x.png", layer.name)))?;
    }
    let mut sheet = RgbaImage::from_pixel(640 * 3, 360, Rgba([0, 0, 0, 255]));
    for (i, camera) in [0.0, 160.0, 640.0].into_iter().enumerate() {
        let image = preview(&layers, camera);
        image.save(out.join(format!("camera-{}.png", camera as i32)))?;
        image::imageops::replace(&mut sheet, &image, i as i64 * 640, 0);
    }
    sheet.save(out.join("parallax.png"))?;
    resize(&sheet, 3840, 720, FilterType::Nearest).save(out.join("parallax-2x.png"))?;
    println!("{} (camera offsets: 0, 160, 640 pixels)", out.display());
    Ok(())
}
