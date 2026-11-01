//! WAV export and fixed-scale waveform, seam and logarithmic spectrogram diagnostics.
use assetgen::music::{self, Pcm, SAMPLE_RATE, STEMS};
use image::{Rgb, RgbImage};
use std::{f64::consts::TAU, fs, path::Path};

pub fn render() -> crate::Result<()> {
    let assets = crate::root().join("assets/music/ballroom");
    let out = crate::root().join("target/music");
    fs::create_dir_all(&assets)?;
    fs::create_dir_all(&out)?;
    let stems = music::render();
    println!("Ballroom: 120 BPM, D minor, 4/4, 32 bars; stereo 44.1 kHz / 16-bit");
    println!("stem       seconds  peak dBFS  RMS dBFS  seam L/R (PCM)");
    for (name, pcm) in STEMS.into_iter().zip(&stems) {
        music::write_wav(&assets.join(format!("{name}.wav")), pcm)?;
        diagnostics(name, pcm, &out)?;
    }
    let mix = music::mix(&stems);
    music::write_wav(&out.join("full-mix.wav"), &mix)?;
    diagnostics("full-mix", &mix, &out)?;
    println!("Waveforms: top = full loop at ±1; bottom = last/first 10 ms at ±0.2 (seam in red).");
    println!("Spectrograms: time left to right; 30 Hz at bottom, 16 kHz at top; -90..-18 dBFS.");
    println!("Diagnostics: {}", out.display());
    let assets = crate::root().join("assets/music/dancer");
    let out = out.join("dancer");
    fs::create_dir_all(&assets)?;
    fs::create_dir_all(&out)?;
    let stems = music::render_dancer();
    println!("Dancer: 132 BPM, D minor, 4/4, 40 bars");
    for (name, pcm) in music::DANCER_STEMS.into_iter().zip(&stems) {
        music::write_wav(&assets.join(format!("{name}.wav")), pcm)?;
        diagnostics(name, pcm, &out)?;
    }
    for phase in 1..=3 {
        let layers: [Pcm; 8] = std::array::from_fn(|i| {
            if i < [3,5,8][phase-1] { stems[i].clone() } else { vec![[0;2]; music::DANCER_FRAMES] }
        });
        let mix = music::mix(&layers);
        music::write_wav(&out.join(format!("phase-{phase}.wav")), &mix)?;
        diagnostics(&format!("phase-{phase}"), &mix, &out)?;
    }
    println!("Dancer diagnostics: {}", out.display());
    let assets = crate::root().join("assets/music/doorman");
    let out = crate::root().join("target/music/doorman");
    fs::create_dir_all(&assets)?;
    fs::create_dir_all(&out)?;
    let stems = music::render_doorman();
    println!("Doorman: 104 BPM, C minor, 4/4, 32 bars");
    for (name, pcm) in music::DOORMAN_STEMS.into_iter().zip(&stems) {
        music::write_wav(&assets.join(format!("{name}.wav")), pcm)?;
        diagnostics(name, pcm, &out)?;
    }
    for (phase, count) in music::DOORMAN_LAYERS.into_iter().enumerate() {
        let layers: [Pcm; 8] = std::array::from_fn(|i| {
            if i < count { stems[i].clone() } else { vec![[0; 2]; music::DOORMAN_FRAMES] }
        });
        let mix = music::mix(&layers);
        music::write_wav(&out.join(format!("phase-{}.wav", phase + 1)), &mix)?;
        diagnostics(&format!("phase-{}", phase + 1), &mix, &out)?;
    }
    println!("Doorman diagnostics: {}", out.display());
    Ok(())
}

fn diagnostics(name: &str, pcm: &Pcm, out: &Path) -> crate::Result<()> {
    let (length, peak, rms) = music::levels(pcm);
    let seam = std::array::from_fn::<_, 2, _>(|c| i32::from(pcm[0][c]) - i32::from(pcm[pcm.len()-1][c]));
    println!("{name:<10} {length:7.3} {peak:10.2} {rms:9.2}  {}/{}", seam[0], seam[1]);
    waveform(pcm).save(out.join(format!("{name}-waveform.png")))?;
    spectrogram(pcm).save(out.join(format!("{name}-spectrogram.png")))?;
    Ok(())
}

fn waveform(pcm: &Pcm) -> RgbImage {
    let mut image = RgbImage::from_pixel(1000, 440, Rgb([16, 19, 28]));
    for center in [110, 330] {
        for x in 0..1000 { image.put_pixel(x, center, Rgb([65, 69, 81])); }
    }
    for c in 0..2 {
        let color = if c == 0 { Rgb([238, 188, 116]) } else { Rgb([93, 198, 220]) };
        for x in 0..1000 {
            let start = x * pcm.len() / 1000;
            let end = ((x + 1) * pcm.len() / 1000).max(start + 1);
            let (lo, hi) = pcm[start..end].iter().fold((0_i16, 0_i16), |(lo, hi), f| (lo.min(f[c]), hi.max(f[c])));
            for y in (110.0 - f64::from(hi) / 32768.0 * 100.0) as u32
                ..=(110.0 - f64::from(lo) / 32768.0 * 100.0) as u32 {
                image.put_pixel(x as u32, y.min(219), color);
            }
            let offset = x as isize * 882 / 1000 - 441;
            let index = offset.rem_euclid(pcm.len() as isize) as usize;
            let y = (330.0 - f64::from(pcm[index][c]) / 32768.0 * 500.0).clamp(221.0, 439.0);
            image.put_pixel(x as u32, y as u32, color);
        }
    }
    for y in 220..440 { image.put_pixel(500, y, Rgb([171, 69, 87])); }
    image
}

// In-place radix-two FFT, used only by the offline diagnostic plot.
fn fft(values: &mut [[f64; 2]]) {
    let n = values.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 { j ^= bit; bit >>= 1; }
        j ^= bit;
        if i < j { values.swap(i, j); }
    }
    let mut size = 2;
    while size <= n {
        let angle = -TAU / size as f64;
        let step = [angle.cos(), angle.sin()];
        for start in (0..n).step_by(size) {
            let mut w = [1.0, 0.0];
            for k in 0..size / 2 {
                let a = values[start + k];
                let b = values[start + k + size / 2];
                let product = [b[0] * w[0] - b[1] * w[1], b[0] * w[1] + b[1] * w[0]];
                values[start + k] = [a[0] + product[0], a[1] + product[1]];
                values[start + k + size / 2] = [a[0] - product[0], a[1] - product[1]];
                w = [w[0] * step[0] - w[1] * step[1], w[0] * step[1] + w[1] * step[0]];
            }
        }
        size *= 2;
    }
}

fn spectrogram(pcm: &Pcm) -> RgbImage {
    const N: usize = 4096;
    let mut image = RgbImage::new(1000, 384);
    let mut bins = vec![[0.0; 2]; N];
    let mut energy = vec![0.0_f64; N / 2];
    for x in 0..1000 {
        energy.fill(0.0);
        for c in 0..2 {
            for (i, bin) in bins.iter_mut().enumerate() {
                let index = (x * pcm.len() / 1000 + i + pcm.len() - N / 2) % pcm.len();
                let window = 0.5 - 0.5 * (TAU * i as f64 / N as f64).cos();
                *bin = [f64::from(pcm[index][c]) / 32768.0 * window, 0.0];
            }
            fft(&mut bins);
            for (e, bin) in energy.iter_mut().zip(&bins) {
                *e += (bin[0].powi(2) + bin[1].powi(2)) * 0.5;
            }
        }
        for y in 0..384 {
            let frequency = 30.0 * (16000.0_f64 / 30.0).powf((383 - y) as f64 / 383.0);
            let bin = (frequency * N as f64 / f64::from(SAMPLE_RATE)).round() as usize;
            let amplitude = energy[bin.min(N / 2 - 1)].sqrt() * 4.0 / N as f64;
            let level = ((20.0 * amplitude.max(1e-9).log10() + 90.0) / 72.0).clamp(0.0, 1.0);
            image.put_pixel(x as u32, y, Rgb([
                (15.0 + 240.0 * level.powf(1.4)) as u8,
                (18.0 + 202.0 * level.powf(2.3)) as u8,
                (30.0 + 95.0 * (std::f64::consts::PI * level).sin()) as u8,
            ]));
        }
    }
    image
}
