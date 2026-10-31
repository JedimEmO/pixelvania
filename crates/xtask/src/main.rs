//! Offline PNG and GIF previews and pinned JavaScript reference generation.
use image::{
    Delay, Frame as GifFrame, RgbaImage,
    codecs::gif::{GifEncoder, Repeat},
    imageops::{FilterType, resize},
};
use pixelfx::{
    Effect, EffectId, Frame, Options,
    act::{pose_at, timing},
    actor::Actor,
    painter::Painter,
    rig::{Pose, Rig},
    sim::Layer,
};
use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
const COMMIT: &str = "a84159688af3ca37a4be05a0d422b94451b2e24e";
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("command {cmd:?} failed: {status}").into());
    }
    Ok(())
}
fn output(cmd: &mut Command) -> Result<String> {
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(format!(
            "command {cmd:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}
fn golden() -> Result<()> {
    let dir = std::env::var_os("PIXELFX_JS")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/pixelfx-js"));
    if !dir.exists() {
        fs::create_dir_all(dir.parent().ok_or("missing JS parent")?)?;
        run(Command::new("git")
            .args([
                "clone",
                "--no-checkout",
                "https://github.com/patch-gg/pixelfx",
            ])
            .arg(&dir))?;
        run(Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["fetch", "origin", COMMIT]))?;
        run(Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["checkout", "--detach", COMMIT]))?;
    }
    let head = output(
        Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"]),
    )?;
    if head != COMMIT {
        return Err(format!("{} is at {head}; expected {COMMIT}", dir.display()).into());
    }
    let dirty = output(Command::new("git").arg("-C").arg(&dir).args([
        "status",
        "--porcelain",
        "--untracked-files=no",
        "--",
        "src",
        "tools",
    ]))?;
    if !dirty.is_empty() {
        return Err("pixelfx JS source has tracked changes; reference generation requires the pinned original".into());
    }
    let out = root().join("crates/pixelfx/tests/golden");
    let mut child = Command::new("node")
        .args(["--input-type=module", "-"])
        .arg(fs::canonicalize(dir)?)
        .arg(&out)
        .stdin(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing node stdin")?
        .write_all(include_bytes!("golden.mjs"))?;
    let status = child.wait()?;
    if !status.success() {
        return Err(format!("golden renderer failed: {status}").into());
    }
    println!("References: {}", out.display());
    Ok(())
}
fn rgba(frame: &Frame) -> RgbaImage {
    RgbaImage::from_raw(frame.width, frame.height, frame.composite([12, 11, 16]))
        .expect("frame dimensions")
}
fn sheet(images: &[RgbaImage], path: &Path) -> Result<()> {
    let indices: Vec<_> = if images.len() <= 16 {
        (0..images.len()).collect()
    } else {
        (0..16)
            .map(|i| ((i * (images.len() - 1)) as f64 / 15.0).round() as usize)
            .collect()
    };
    let (w, h) = images[0].dimensions();
    let scale = 2;
    let cols = 4;
    let rows = (indices.len() as u32).div_ceil(cols);
    let mut out = RgbaImage::from_pixel(
        cols * (w * scale + 8) + 8,
        rows * (h * scale + 8) + 8,
        image::Rgba([24, 22, 30, 255]),
    );
    for (j, &i) in indices.iter().enumerate() {
        let img = resize(&images[i], w * scale, h * scale, FilterType::Nearest);
        image::imageops::replace(
            &mut out,
            &img,
            (8 + j as u32 % cols * (w * scale + 8)) as i64,
            (8 + j as u32 / cols * (h * scale + 8)) as i64,
        );
    }
    out.save(path)?;
    println!("{}", path.display());
    Ok(())
}
fn fx(id: EffectId, seed: u32, out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let m = id.metadata();
    let mut effect = Effect::new(id, seed, Options::default());
    let mut f = Frame::new(m.size[0], m.size[1]);
    let actor = matches!(
        id,
        EffectId::SwordSlash | EffectId::HammerSmash | EffectId::HeavyLanding
    )
    .then(|| Actor::stock(true));
    let n = (m.duration * 30.0).round() as usize;
    let mut images = vec![];
    for i in 0..=n {
        if i > 0 {
            effect.update(1.0 / 30.0, &|_| 0.0);
        }
        f.clear();
        effect.draw_layer(&mut f, m.origin, Layer::Back);
        if let Some(actor) = &actor {
            let mut p = Painter::new(&mut f);
            p.place(m.origin[0], m.origin[1], false, 0.0);
            actor.draw(&mut p, 1.0);
        }
        effect.draw_layer(&mut f, m.origin, Layer::Front);
        images.push(rgba(&f));
    }
    let name = id.name().replace('/', "-");
    sheet(&images, &out.join(format!("{name}.png")))?;
    let file = fs::File::create(out.join(format!("{name}.gif")))?;
    let mut gif = GifEncoder::new(file);
    gif.set_repeat(Repeat::Infinite)?;
    for img in images {
        let img = resize(&img, img.width() * 3, img.height() * 3, FilterType::Nearest);
        gif.encode_frame(GifFrame::from_parts(
            img,
            0,
            0,
            Delay::from_numer_denom_ms(1000, 30),
        ))?;
    }
    Ok(())
}
/// Renders the player's animation to contact sheets at 2x: one row of eight steps for each
/// movement state, and one row for each sabre swing through wind-up, release and recovery.
fn player_sheet() -> Result<()> {
    use anim::swing::{Phase, Swing, swing_pose};
    let out = root().join("target/rig");
    fs::create_dir_all(&out)?;
    let moves: Vec<(&str, Vec<anim::Pose>)> = anim::State::ALL
        .iter()
        .map(|s| {
            let poses = (0..8)
                .map(|i| anim::pose_for(*s, i as f64 / anim::FPS, i as f64 / 8.0))
                .collect();
            (s.name(), poses)
        })
        .collect();
    draw_rows(&moves, &out.join("player.png"))?;
    let steps = [
        (Phase::Windup, 0.0),
        (Phase::Windup, 0.35),
        (Phase::Windup, 0.7),
        (Phase::Windup, 1.0),
        (Phase::Active, 0.0),
        (Phase::Active, 1.0),
        (Phase::Recovery, 0.4),
        (Phase::Recovery, 1.0),
    ];
    let swings: Vec<(&str, Vec<anim::Pose>)> = Swing::ALL
        .iter()
        .map(|s| {
            (
                s.name(),
                steps.iter().map(|&(p, u)| swing_pose(*s, p, u)).collect(),
            )
        })
        .collect();
    draw_rows(&swings, &out.join("player-sabre.png"))?;
    guests_sheet(&out)
}

/// One row per guest class: standing, its swing, and every reaction.
fn guests_sheet(out: &Path) -> Result<()> {
    use anim::guest::{Class, Reaction, arm, reaction_pose, rig, scale};
    use anim::swing::{Phase, Swing, swing_pose};
    for class in Class::ALL {
        let k = class.scale();
        let swing = if class == Class::Heavy {
            Swing::Combo3
        } else {
            Swing::Combo1
        };
        let mut poses = vec![
            anim::pose_for(anim::State::Idle, 0.0, 0.0),
            swing_pose(swing, Phase::Windup, 1.0),
            swing_pose(swing, Phase::Active, 0.0),
            swing_pose(swing, Phase::Active, 1.0),
        ];
        poses.extend(Reaction::ALL.iter().map(|r| reaction_pose(*r, 0.1)));
        let poses: Vec<anim::Pose> = poses.iter().map(|p| arm(scale(p, k), class)).collect();
        draw_rows_with(
            &rig(class),
            &[(class.name(), poses)],
            &out.join(format!("guest-{}.png", class.name())),
        )?;
    }
    Ok(())
}

fn draw_rows(rows: &[(&str, Vec<anim::Pose>)], path: &Path) -> Result<()> {
    draw_rows_with(&anim::player_rig(), rows, path)
}

fn draw_rows_with(rig: &anim::Rig, rows: &[(&str, Vec<anim::Pose>)], path: &Path) -> Result<()> {
    let (w, h, scale) = (80u32, 64u32, 2u32);
    let cols = rows.iter().map(|r| r.1.len()).max().unwrap_or(1) as u32;
    let bg = image::Rgba([24, 20, 34, 255]);
    let ground = image::Rgba([60, 52, 80, 255]);
    let mut sheet = RgbaImage::from_pixel(w * cols * scale, h * rows.len() as u32 * scale, bg);
    for (row, (name, poses)) in rows.iter().enumerate() {
        for (col, pose) in poses.iter().enumerate() {
            let mut f = Frame::new(w, h);
            anim::draw(rig, pose, &mut f, [36.0, 56.0]);
            let img = rgba(&f);
            for (x, y, px) in img.enumerate_pixels() {
                // The ground line shows through empty pixels, so feet can be checked against it.
                let c = if px[3] > 0 {
                    *px
                } else if y == 56 {
                    ground
                } else {
                    continue;
                };
                for dy in 0..scale {
                    for dx in 0..scale {
                        let sx = (col as u32 * w + x) * scale + dx;
                        let sy = (row as u32 * h + y) * scale + dy;
                        sheet.put_pixel(sx, sy, c);
                    }
                }
            }
        }
        println!("{row:>2}: {name}");
    }
    sheet.save(path)?;
    println!("{}", path.display());
    Ok(())
}

fn rig(character: &str) -> Result<()> {
    if character == "player" {
        return player_sheet();
    }
    if character != "stock" {
        return Err(format!("unknown character {character}; available: stock, player").into());
    }
    let out = root().join("target/rig");
    fs::create_dir_all(&out)?;
    let rig = Rig::default();
    let render = |poses: Vec<Pose>, name: &str| -> Result<()> {
        let mut images = vec![];
        for pose in poses {
            let mut f = Frame::new(160, 144);
            let mut p = Painter::new(&mut f);
            p.place(
                80.0 + pixelfx::math::round(pose.x),
                116.0 + pixelfx::math::round(pose.y),
                false,
                0.0,
            );
            rig.render(&pose).draw(&mut p);
            images.push(rgba(&f));
        }
        sheet(&images, &out.join(format!("{character}-{name}.png")))
    };
    render(vec![Pose::default(), Pose::ready()], "stances")?;
    render(
        (0..16)
            .map(|i| Pose {
                yaw: i as f64 / 16.0 * std::f64::consts::TAU,
                ..Pose::ready()
            })
            .collect(),
        "turn",
    )?;
    for id in [
        EffectId::HeavyLanding,
        EffectId::SwordSlash,
        EffectId::HammerSmash,
    ] {
        let a = timing(id).unwrap();
        let from = -a.windup;
        let to = a.duration + a.recovery;
        render(
            (0..=((to - from) * 30.0).ceil() as usize)
                .map(|i| pose_at(&a.poses, from + i as f64 / 30.0))
                .collect(),
            id.name().rsplit('/').next().unwrap(),
        )?;
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("golden") if args.len() == 1 => golden(),
        Some("rig") if args.len() <= 2 => rig(args.get(1).map_or("stock", String::as_str)),
        Some("fx") => {
            let id=args.get(1).and_then(|s|EffectId::parse(s)).ok_or("fx requires an id: impacts/hit-spark, impacts/heavy-landing, melee/sword-slash, melee/hammer-smash, spawn/poof, dust")?;
            let mut seed = id.metadata().seed;
            let mut out = root().join("target/fx");
            let mut i = 2;
            while i < args.len() {
                let v = args.get(i + 1).ok_or("option needs a value")?;
                match args[i].as_str() {
                    "--seed" => seed = v.parse()?,
                    "--out" => out = PathBuf::from(v),
                    _ => return Err(format!("unknown option {}", args[i]).into()),
                }
                i += 2;
            }
            fx(id, seed, &out)
        }
        _ => Err(
            "usage: cargo xtask fx <id> [--seed N] [--out DIR] | rig [stock|player] | golden"
                .into(),
        ),
    }
}
