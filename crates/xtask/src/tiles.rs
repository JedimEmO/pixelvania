//! Tile contact sheets and whole-room previews.
use super::{Result, root};
use assetgen::{MATERIALS, Neighbours, SIZE, VARIANTS, rasterize, tile};
use image::{Rgba, RgbaImage, imageops};
use std::{fs, path::Path};
use world::Room;

fn save_pair(image: &RgbaImage, path: &Path) -> Result<()> {
    image.save(path)?;
    let large = imageops::resize(
        image,
        image.width() * 2,
        image.height() * 2,
        imageops::FilterType::Nearest,
    );
    large.save(path.with_file_name(format!(
        "{}-2x.png",
        path.file_stem().unwrap().to_string_lossy()
    )))?;
    Ok(())
}

fn linear(s: u8) -> f32 {
    let s = f32::from(s) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb(l: f32) -> u8 {
    let s = if l <= 0.0031308 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

// Flat neutral light at 0.6 (one of the shader's five bands), plus its 0.55 rim.
// No glow, lamps or grading: this view checks material shapes under the same light.
fn dim_preview(albedo: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(albedo.width(), albedo.height(), |x, y| {
        let p = albedo.get_pixel(x, y);
        if p[3] == 0 {
            return Rgba([9, 8, 12, 255]);
        }
        let rim = [(0, -1), (1, 0), (0, 1), (-1, 0)]
            .into_iter()
            .any(|(dx, dy)| {
                let nx = (x as i64 + dx).clamp(0, i64::from(albedo.width()) - 1) as u32;
                let ny = (y as i64 + dy).clamp(0, i64::from(albedo.height()) - 1) as u32;
                albedo.get_pixel(nx, ny)[3] == 0
            });
        let light = 0.6 * if rim { 1.0 + 0.55 * 4.0 } else { 1.0 };
        Rgba([
            srgb(linear(p[0]) * light),
            srgb(linear(p[1]) * light),
            srgb(linear(p[2]) * light),
            255,
        ])
    })
}

pub fn render() -> Result<()> {
    let out = root().join("target/tiles");
    fs::create_dir_all(out.join("rooms"))?;
    // Each panel: eight interior variants, eight exposed variants, eight corner
    // cases, then a connected 8 × 4 wall with a notch and a material boundary.
    let panel_w = VARIANTS * 20 + 8;
    let mut sheet = RgbaImage::new(panel_w * MATERIALS.len() as u32, 140);
    for (i, material) in MATERIALS.iter().enumerate() {
        let ox = i as u32 * panel_w + 4;
        for v in 0..VARIANTS {
            for (row, mask) in [255, 0, [254, 251, 239, 191, 253, 247, 223, 127][v as usize]]
                .into_iter()
                .enumerate()
            {
                imageops::replace(
                    &mut sheet,
                    &tile(material, v, Neighbours(mask)),
                    i64::from(ox + v * 20),
                    4 + row as i64 * 20,
                );
            }
        }
        let sample = Room::parse(&format!(
            "[legend]\n# {material}\n= {}\n[tiles]\n########\n####..##\n####..##\n##==####\n",
            MATERIALS[(i + 1) % MATERIALS.len()]
        ))?;
        imageops::replace(&mut sheet, &rasterize(&sample), i64::from(ox), 72);

        let mut masks = RgbaImage::new(320, 320);
        for mask in 0..=255u8 {
            imageops::replace(
                &mut masks,
                &tile(material, 0, Neighbours(mask)),
                i64::from(mask % 16) * 20 + 2,
                i64::from(mask / 16) * 20 + 2,
            );
        }
        save_pair(
            &dim_preview(&masks),
            &out.join(format!("{material}-edges.png")),
        )?;
    }
    save_pair(&sheet, &out.join("materials-albedo.png"))?;
    save_pair(&dim_preview(&sheet), &out.join("materials.png"))?;

    let mut paths: Vec<_> = fs::read_dir(root().join("assets/rooms"))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.sort();
    for path in paths {
        if !path.extension().is_some_and(|e| e == "room") {
            continue;
        }
        let room = Room::parse(&fs::read_to_string(&path)?)?;
        let albedo = rasterize(&room);
        let name = path.file_stem().unwrap().to_string_lossy();
        albedo.save(out.join("rooms").join(format!("{name}-albedo.png")))?;
        save_pair(
            &dim_preview(&albedo),
            &out.join("rooms").join(format!("{name}.png")),
        )?;
        // A screen-sized crop keeps the very wide lab inspectable at native scale.
        let crop = imageops::crop_imm(
            &dim_preview(&albedo),
            0,
            albedo.height().saturating_sub(360),
            albedo.width().min(640),
            albedo.height().min(360),
        )
        .to_image();
        save_pair(&crop, &out.join("rooms").join(format!("{name}-detail.png")))?;
        println!(
            "{}: {} × {} pixels",
            path.display(),
            room.width as u32 * SIZE,
            room.height as u32 * SIZE
        );
    }
    fs::write(
        out.join("README.txt"),
        "materials.png: stone, wood, velvet, gilt, left to right.\nEach panel: 8 interior variants; 8 isolated variants; missing N, E, S, W, NE, SE, SW, NW; connected wall.\n*-edges.png: all 256 neighbour masks, row-major, seed 0. Bits clockwise N, NE, E, SE, S, SW, W, NW; 1 means same material.\n*-albedo.png: unlit sRGB, empty pixels transparent.\nOther previews: flat 0.6 linear light, 0.55 rim, dark background. No lamps or glow.\n*-2x.png: nearest-neighbour enlargement. rooms/*-detail.png: bottom-left screen crop.\n",
    )?;
    println!("Tile previews: {}", out.display());
    Ok(())
}
