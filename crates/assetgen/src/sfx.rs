//! Deterministic offline effects, PCM WAV export and diagnostic plots.
use image::{Rgb, RgbImage};
use std::{error::Error, f64::consts::TAU, fs, path::Path};

pub const SAMPLE_RATE: u32 = 44_100;

/// Recipe order also fixes each sound's noise seed.
pub const SOUNDS: [(&str, &str, f64, f64); 16] = [
    ("SwingLight", "swing_light", 0.16, -6.0),
    ("SwingHeavy", "swing_heavy", 0.30, -4.5),
    ("BottleSwing", "bottle_swing", 0.34, -4.5),
    ("HitLight", "hit_light", 0.18, -3.0),
    ("HitHeavy", "hit_heavy", 0.34, -3.0),
    ("HitCharged", "hit_charged", 0.39, -3.0),
    ("BottleHit", "bottle_hit", 0.36, -3.0),
    ("Parry", "parry", 0.38, -3.0),
    ("WallSplat", "wall_splat", 0.32, -3.0),
    ("Finisher", "finisher", 1.10, -3.0),
    ("Jump", "jump", 0.16, -8.0),
    ("Land", "land", 0.18, -7.0),
    ("LandHeavy", "land_heavy", 0.34, -3.0),
    ("Lunge", "lunge", 0.18, -6.0),
    ("Footstep", "footstep", 0.10, -16.0),
    ("GuestWindup", "guest_windup", 0.26, -9.0),
];

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX) * 2.0 - 1.0
    }
}

#[derive(Default)]
struct LowPass(f64);
impl LowPass {
    fn tick(&mut self, x: f64, hz: f64) -> f64 {
        self.0 += (1.0 - (-TAU * hz / f64::from(SAMPLE_RATE)).exp()) * (x - self.0);
        self.0
    }
}

fn tone(t: f64, hz: f64, decay: f64) -> f64 {
    (TAU * hz * t).sin() * (-t / decay).exp()
}

// Integrated exponential pitch drop: 120 Hz at impact, settling around 48 Hz.
fn thump(t: f64, decay: f64) -> f64 {
    (TAU * (48.0 * t + 72.0 * 0.012 * (1.0 - (-t / 0.012).exp()))).sin() * (-t / decay).exp()
}

/// Renders one recipe to signed 16-bit mono samples, including its mix level.
pub fn render(index: usize) -> Vec<i16> {
    let (_, name, duration, peak_db) = SOUNDS[index];
    let count = (duration * f64::from(SAMPLE_RATE)).round() as usize;
    let mut noise = Noise(0x5f37_59df ^ (index as u32 + 1).wrapping_mul(0x9e37_79b9));
    let (mut low, mut high, mut dc) = (LowPass::default(), LowPass::default(), LowPass::default());
    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f64 / f64::from(SAMPLE_RATE);
        let u = t / duration;
        let n = noise.next();
        let dark = low.tick(n, 650.0);
        let air = n - high.tick(n, 1800.0);
        let crack = air * (-t / 0.006).exp();
        let body = dark * (-t / 0.045).exp();
        let sweep = (std::f64::consts::PI * u).sin().powi(2);
        let x = match name {
            "swing_light" => air * sweep,
            "swing_heavy" => dark * sweep + 0.08 * air * sweep,
            "bottle_swing" => {
                dark * sweep + 0.10 * tone(t, 210.0, 0.25) * (TAU * 17.0 * t).sin().powi(2) * sweep
            }
            "hit_light" => 0.6 * crack + 0.45 * tone(t, 310.0, 0.025) + 0.3 * body,
            "hit_heavy" => 0.45 * crack + thump(t, 0.095) + 0.3 * body,
            "hit_charged" => {
                0.65 * crack + thump(t, 0.13) + 0.35 * body + 0.16 * tone(t, 143.0, 0.045)
            }
            "bottle_hit" => {
                0.75 * thump(t, 0.11)
                    + 0.4 * tone(t, 430.0, 0.035)
                    + 0.2 * tone(t, 713.0, 0.022)
                    + 0.2 * crack
            }
            "parry" => {
                tone(t, 1760.0, 0.09)
                    + 0.45 * tone(t, 2873.0, 0.06)
                    + 0.25 * tone(t, 4211.0, 0.035)
                    + 0.08 * crack
            }
            "wall_splat" => {
                0.65 * thump(t, 0.075)
                    + 0.8 * body
                    + 0.3 * air * (-t / 0.035).exp() * (TAU * 93.0 * t).sin().abs()
            }
            "finisher" => {
                0.65 * crack
                    + 0.7 * tone(t, 380.0, 0.018)
                    + 0.14 * air * (1.0 - (-t / 0.03).exp()) * (-t / 0.27).exp()
            }
            "jump" => 0.6 * dark * sweep + 0.16 * (TAU * (240.0 * t + 900.0 * t * t)).sin() * sweep,
            "land" => body + 0.22 * tone(t, 150.0, 0.03),
            "land_heavy" => 0.8 * thump(t, 0.09) + body + 0.18 * crack,
            "lunge" => (0.5 * air + dark) * sweep * (-t / 0.065).exp(),
            "footstep" => 0.6 * body + 0.12 * tone(t, 480.0, 0.012),
            "guest_windup" => {
                (TAU * (620.0 * t + 500.0 * t * t)).sin() * sweep + 0.08 * air * sweep
            }
            _ => unreachable!(),
        };
        // Light impacts have no sub layer. A two-pole high-pass also removes
        // incidental low-frequency noise from their transient and body.
        let cutoff = if name == "hit_light" || name == "swing_light" {
            180.0
        } else {
            15.0
        };
        let x = x - dc.tick(x, cutoff);
        let fade = (t / 0.001).min(1.0)
            * ((duration - t - 1.0 / f64::from(SAMPLE_RATE)) / 0.012).clamp(0.0, 1.0);
        samples.push(x * fade);
    }
    if name == "hit_light" || name == "swing_light" {
        let mut filter = LowPass::default();
        for x in &mut samples {
            *x -= filter.tick(*x, 180.0);
        }
        // Fade after filtering too, so the final sample is exactly silent.
        for (i, x) in samples.iter_mut().enumerate() {
            *x *= ((count - 1 - i) as f64 / (0.008 * f64::from(SAMPLE_RATE))).min(1.0);
        }
    }
    let peak = samples.iter().fold(0.0_f64, |p, x| p.max(x.abs()));
    let gain = 10.0_f64.powf(peak_db / 20.0) / peak;
    samples
        .into_iter()
        .map(|x| (x * gain * f64::from(i16::MAX)).round() as i16)
        .collect()
}

fn floats(samples: &[i16]) -> impl Iterator<Item = f64> + '_ {
    samples.iter().map(|&x| f64::from(x) / f64::from(i16::MAX))
}

/// Mean-square output of a two-pole 80 Hz low-pass, including its decay tail.
pub fn sub_energy(samples: &[i16]) -> f64 {
    let (mut a, mut b) = (LowPass::default(), LowPass::default());
    floats(samples)
        .chain(std::iter::repeat_n(0.0, 4410))
        .map(|x| b.tick(a.tick(x, 80.0), 80.0).powi(2))
        .sum::<f64>()
        / samples.len() as f64
}

/// Returns duration in seconds, peak dBFS and whole-file RMS dBFS.
pub fn levels(samples: &[i16]) -> (f64, f64, f64) {
    let peak = floats(samples).fold(0.0_f64, |p, x| p.max(x.abs()));
    let energy = floats(samples).map(|x| x * x).sum::<f64>() / samples.len() as f64;
    (
        samples.len() as f64 / f64::from(SAMPLE_RATE),
        20.0 * peak.log10(),
        10.0 * energy.log10(),
    )
}

/// Top: waveform; bottom: 80 Hz low-pass waveform. Both use the same amplitude scale.
fn plot(samples: &[i16], path: &Path) -> Result<(), image::ImageError> {
    let mut img = RgbImage::from_pixel(960, 360, Rgb([18, 20, 29]));
    let (mut a, mut b) = (LowPass::default(), LowPass::default());
    let raw: Vec<_> = floats(samples).collect();
    let sub: Vec<_> = raw.iter().map(|&x| b.tick(a.tick(x, 80.0), 80.0)).collect();
    for (data, center, color) in [
        (&raw, 90, Rgb([230, 187, 120])),
        (&sub, 270, Rgb([104, 202, 218])),
    ] {
        for x in 0..960 {
            img.put_pixel(x, center, Rgb([65, 66, 79]));
            let start = x as usize * data.len() / 960;
            let end = ((x as usize + 1) * data.len() / 960).max(start + 1);
            let (lo, hi) = data[start..end]
                .iter()
                .fold((0.0_f64, 0.0_f64), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            for y in
                (f64::from(center) - hi * 100.0) as u32..=(f64::from(center) - lo * 100.0) as u32
            {
                img.put_pixel(x, y, color);
            }
        }
    }
    img.save(path)
}

/// Rebuilds shipped WAVs, the game's enum/embed catalog and waveform diagnostics.
pub fn render_all(assets: &Path, diagnostics: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(assets)?;
    fs::create_dir_all(diagnostics)?;
    let mut catalog = String::from("// Generated by cargo xtask sfx.\nsfx_catalog! {\n");
    println!("sound             seconds  peak dBFS  RMS dBFS  sub80 dBFS");
    for (i, &(variant, name, _, _)) in SOUNDS.iter().enumerate() {
        let samples = render(i);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(assets.join(format!("{name}.wav")), spec)?;
        for &sample in &samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        plot(&samples, &diagnostics.join(format!("{name}.png")))?;
        let (length, peak, rms) = levels(&samples);
        println!(
            "{name:<18} {length:5.3} {peak:10.2} {rms:9.2} {:11.2}",
            10.0 * sub_energy(&samples).log10()
        );
        catalog.push_str(&format!("    {variant} => \"{name}\",\n"));
    }
    catalog.push_str("}\n");
    fs::write(assets.join("catalog.rs"), catalog)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_repeatably_and_matches_shipped_pcm() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sfx");
        for (i, &(_, name, duration, _)) in SOUNDS.iter().enumerate() {
            let pcm = render(i);
            assert_eq!(pcm, render(i), "{name}");
            assert_eq!(
                pcm.len(),
                (duration * f64::from(SAMPLE_RATE)).round() as usize
            );
            assert_eq!((pcm[0], pcm[pcm.len() - 1]), (0, 0), "{name}");
            let mut wav = hound::WavReader::open(assets.join(format!("{name}.wav"))).unwrap();
            assert_eq!(wav.spec().sample_rate, SAMPLE_RATE);
            assert_eq!(wav.spec().channels, 1);
            assert_eq!(wav.spec().bits_per_sample, 16);
            assert_eq!(
                wav.samples::<i16>().collect::<Result<Vec<_>, _>>().unwrap(),
                pcm
            );
        }
    }
    #[test]
    fn heavy_hits_have_sub_bass_and_light_hits_do_not() {
        let light = render(3);
        let light_sub = sub_energy(&light);
        let light_total = 10.0_f64.powf(levels(&light).2 / 10.0);
        assert!(light_sub / light_total < 0.01);
        for i in [4, 5, 6, 8, 12] {
            let heavy = render(i);
            let sub = sub_energy(&heavy);
            let total = 10.0_f64.powf(levels(&heavy).2 / 10.0);
            assert!(sub > light_sub * 100.0, "{}", SOUNDS[i].1);
            assert!(sub / total > 0.25, "{}", SOUNDS[i].1);
        }
    }
}
