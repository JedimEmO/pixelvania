//! The ballroom: 32 bars of worn Italo disco in D minor, rendered as circular stems.
use std::{f64::consts::TAU, path::Path};

pub const SAMPLE_RATE: u32 = 44_100;
pub const BPM: usize = 120;
pub const BARS: usize = 32;
pub const FRAMES: usize = SAMPLE_RATE as usize * 60 / BPM * 4 * BARS;
pub const STEMS: [&str; 5] = ["drums", "bass", "harmony", "lead", "texture"];
pub type Pcm = Vec<[i16; 2]>;
type Signal = Vec<[f64; 2]>;
const BEAT: f64 = 60.0 / BPM as f64;

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX) * 2.0 - 1.0
    }
}

fn hz(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((f64::from(note) - 69.0) / 12.0)
}

// Notes include their release; indexing wraps that release into the next turn.
fn event(out: &mut Signal, beat: f64, duration: f64, pan: f64, mut voice: impl FnMut(f64) -> f64) {
    let start = (beat * BEAT * f64::from(SAMPLE_RATE)).round() as usize;
    let count = (duration * f64::from(SAMPLE_RATE)).round() as usize;
    let gains = [((1.0 - pan) * 0.5).sqrt(), ((1.0 + pan) * 0.5).sqrt()];
    for i in 0..count {
        let t = i as f64 / f64::from(SAMPLE_RATE);
        let end = (count - 1 - i) as f64 / f64::from(SAMPLE_RATE);
        let x = voice(t) * (t / 0.008).min(1.0) * (end / 0.025).min(1.0);
        for (channel, gain) in out[(start + i) % FRAMES].iter_mut().zip(gains) {
            *channel += x * gain;
        }
    }
}

// Two passes settle the one-pole filter to the loop's periodic state.
fn low_pass(data: &mut Signal, cutoff: f64) {
    let a = 1.0 - (-TAU * cutoff / f64::from(SAMPLE_RATE)).exp();
    let mut state = [0.0; 2];
    for frame in data.iter() {
        for c in 0..2 {
            state[c] += a * (frame[c] - state[c]);
        }
    }
    for frame in data {
        for (sample, state) in frame.iter_mut().zip(&mut state) {
            *state += a * (*sample - *state);
            *sample = *state;
        }
    }
}

// Parallel damped feedback combs. Warm up on one complete loop, then capture
// the next; even the longest comb has decayed far below 16-bit noise by then.
fn room(data: &mut Signal, wet: f64) {
    let mut reverberation = vec![[0.0; 2]; data.len()];
    for (delay, feedback) in [(1301, 0.68), (1777, 0.65), (2137, 0.63)] {
        for c in 0..2 {
            let mut line = vec![0.0; delay + c * 113];
            let mut position = 0;
            let mut damp = 0.0;
            for pass in 0..2 {
                for (i, frame) in data.iter().enumerate() {
                    let echo = line[position];
                    damp += 0.28 * (echo - damp);
                    line[position] = frame[c] + feedback * damp;
                    position = (position + 1) % line.len();
                    if pass == 1 {
                        reverberation[i][c] += echo * wet / 3.0;
                    }
                }
            }
        }
    }
    for (frame, tail) in data.iter_mut().zip(reverberation) {
        for c in 0..2 {
            frame[c] += tail[c];
        }
    }
}

/// Renders all five stereo stems with one shared gain, targeting a -6 dBFS sum.
pub fn render() -> [Pcm; 5] {
    let mut stems: [Signal; 5] = std::array::from_fn(|_| vec![[0.0; 2]; FRAMES]);
    // Two bars per chord: Dm9, Bbmaj7, Fmaj7, C6, Gm9, Dm9/A, Bbmaj7, A7(b9).
    let chords = [
        (38, [53, 57, 60, 64]),
        (34, [53, 57, 62, 65]),
        (41, [52, 57, 60, 65]),
        (36, [52, 57, 60, 67]),
        (43, [53, 57, 58, 62]),
        (33, [53, 57, 60, 64]),
        (34, [53, 57, 62, 65]),
        (33, [55, 58, 61, 64]),
    ];
    let mut noise = Noise(0xb411_700d);
    for bar in 0..BARS {
        let start = bar as f64 * 4.0;
        let (root, chord) = chords[(bar / 2) % chords.len()];
        // A/A'/B/A'': the middle eight bars leave more air around the toast.
        let sparse = (16..24).contains(&bar);
        for beat in 0..4 {
            event(&mut stems[0], start + beat as f64, 0.30, 0.0, |t| {
                let phase = TAU * (49.0 * t + 55.0 * 0.018 * (1.0 - (-t / 0.018).exp()));
                0.40 * phase.sin() * (-t / 0.075).exp()
            });
            if beat % 2 == 1 {
                let mut brush = 0.0;
                event(&mut stems[0], start + beat as f64, 0.23, -0.16, |t| {
                    let n = noise.next();
                    brush += 0.12 * (n - brush);
                    0.10 * (n - brush) * (-t / 0.045).exp()
                });
            }
            event(&mut stems[0], start + beat as f64 + 0.5, 0.13, 0.35, |t| {
                0.055 * noise.next() * (-t / 0.024).exp()
            });
            for off in [0.0, 0.5] {
                let note = root + if off > 0.0 { 12 } else { 0 };
                event(&mut stems[1], start + beat as f64 + off, 0.23, 0.0, |t| {
                    let phase = TAU * hz(note) * t;
                    0.21 * (phase.sin() + 0.22 * (2.0 * phase).sin()) * (-t / 0.14).exp()
                });
            }
        }
        if bar % 2 == 0 {
            for (v, note) in chord.into_iter().enumerate() {
                event(&mut stems[2], start, 4.8, (v as f64 - 1.5) * 0.35, |t| {
                    let phase = TAU * hz(note) * t;
                    let env = (t / 0.25).min(1.0) * ((4.8 - t) / 1.2).min(1.0);
                    0.064
                        * env
                        * (phase.sin() + 0.45 * (phase * 1.0023).sin() + 0.16 * (phase * 2.0).sin())
                });
            }
        }
        // Soft FM keys give the pad a syncopated disco edge without a sixth stem.
        for off in [0.75, 2.5] {
            for note in chord {
                event(&mut stems[2], start + off, 1.1, -0.25, |t| {
                    let phase = TAU * hz(note + 12) * t;
                    0.025
                        * (phase + 1.1 * (-t / 0.12).exp() * (2.0 * phase).sin()).sin()
                        * (-t / 0.27).exp()
                });
            }
        }
        // Four-note toast D-F-E-A, with chord-aware answers and breathing spaces.
        let melody = match (bar / 2) % 8 {
            0 | 5 => [74, 77, 76, 81],
            1 | 6 => [74, 77, 69, 72],
            2 => [72, 76, 77, 81],
            3 => [76, 79, 76, 72],
            4 => [74, 77, 81, 79],
            _ => [73, 76, 70, 69],
        };
        if bar % 2 == 0 || (!sparse && bar % 4 == 3) {
            for (i, note) in melody.into_iter().enumerate() {
                let beat = [0.5, 1.25, 2.5, 3.5][i];
                let answer = bar % 2 == 1;
                let note = if answer { note - 12 } else { note };
                event(&mut stems[3], start + beat, 1.65, 0.22, |t| {
                    let phase = TAU * hz(note) * t + 0.008 * (TAU * 4.7 * t).sin();
                    let fm = 0.9 * (-t / 0.16).exp() * (2.0 * phase).sin();
                    0.13 * (phase + fm).sin() * (-t / 0.48).exp()
                });
            }
        }
    }
    // No intelligible voices: a periodic bed of band-limited noise, vowel-like
    // partials and occasional tiny dust impulses. Integer cycles preserve phase.
    for (i, frame) in stems[4].iter_mut().enumerate() {
        let u = i as f64 / FRAMES as f64;
        for (c, sample) in frame.iter_mut().enumerate() {
            let n = noise.next();
            let murmur = (TAU * (11231 + 371 * c) as f64 * u).sin()
                * (TAU * 17.0 * u).sin().powi(2)
                + 0.35 * (TAU * 23057.0 * u).sin();
            let dust = if n > 0.998 { 0.035 } else { 0.0 };
            *sample = 0.011 * n + 0.005 * murmur + dust;
        }
    }
    for (index, data) in stems.iter_mut().enumerate() {
        low_pass(data, [5500.0, 700.0, 3400.0, 6500.0, 2200.0][index]);
        // Remove rumble from pads, melody and room noise; keep bass and kick intact.
        let mut lows = data.clone();
        low_pass(&mut lows, [20.0, 20.0, 180.0, 300.0, 220.0][index]);
        for (frame, low) in data.iter_mut().zip(lows) {
            for c in 0..2 {
                frame[c] -= low[c];
            }
        }
        room(data, [0.05, 0.025, 0.22, 0.28, 0.18][index]);
    }
    let mut peak = 0.0_f64;
    for i in 0..FRAMES {
        for c in 0..2 {
            peak = peak.max(stems.iter().map(|s| s[i][c]).sum::<f64>().abs());
        }
    }
    let gain = 10.0_f64.powf(-6.0 / 20.0) / peak;
    stems.map(|s| {
        s.into_iter()
            .map(|frame| frame.map(|v| (v * gain * f64::from(i16::MAX)).round() as i16))
            .collect()
    })
}

/// Returns seconds, peak dBFS and stereo RMS dBFS.
pub fn levels(pcm: &[[i16; 2]]) -> (f64, f64, f64) {
    let peak = pcm
        .iter()
        .flatten()
        .map(|&v| f64::from(v).abs())
        .fold(0.0_f64, f64::max);
    let energy = pcm
        .iter()
        .flatten()
        .map(|&v| f64::from(v).powi(2))
        .sum::<f64>()
        / (pcm.len() * 2) as f64;
    (
        pcm.len() as f64 / f64::from(SAMPLE_RATE),
        20.0 * (peak / f64::from(i16::MAX)).log10(),
        10.0 * (energy / f64::from(i16::MAX).powi(2)).log10(),
    )
}

/// Sums quantized stems, rejecting clipping instead of saturating it away.
pub fn mix(stems: &[Pcm; 5]) -> Pcm {
    (0..stems[0].len())
        .map(|i| {
            std::array::from_fn(|c| {
                let sum = stems.iter().map(|s| i32::from(s[i][c])).sum::<i32>();
                i16::try_from(sum).expect("music mix clips")
            })
        })
        .collect()
}

pub fn write_wav(path: &Path, pcm: &[[i16; 2]]) -> Result<(), hound::Error> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for &sample in pcm.iter().flatten() {
        writer.write_sample(sample)?;
    }
    writer.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_equal_loops_with_headroom() {
        let stems = render();
        assert_eq!(stems, render());
        let sum = mix(&stems);
        assert!((levels(&sum).1 + 6.0).abs() < 0.01);
        for pcm in stems.iter().chain(std::iter::once(&sum)) {
            assert_eq!(pcm.len(), FRAMES);
            assert!(pcm.iter().flatten().all(|v| v.abs() < i16::MAX));
            // Includes PCM quantization and the normal slope of the underlying wave.
            for (&first, &last) in pcm[0].iter().zip(&pcm[FRAMES - 1]) {
                let seam = (i32::from(first) - i32::from(last)).abs();
                assert!(seam < 200, "seam step {seam} exceeds 0.0062 full scale");
            }
        }
    }
}
