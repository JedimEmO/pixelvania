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
fn event(out: &mut Signal, beat: f64, duration: f64, pan: f64, voice: impl FnMut(f64) -> f64) {
    event_seconds(out, beat * BEAT, duration, pan, voice);
}

fn event_seconds(
    out: &mut Signal,
    seconds: f64,
    duration: f64,
    pan: f64,
    mut voice: impl FnMut(f64) -> f64,
) {
    let start = (seconds * f64::from(SAMPLE_RATE)).round() as usize;
    let frames = out.len();
    let count = (duration * f64::from(SAMPLE_RATE)).round() as usize;
    let gains = [((1.0 - pan) * 0.5).sqrt(), ((1.0 + pan) * 0.5).sqrt()];
    for i in 0..count {
        let t = i as f64 / f64::from(SAMPLE_RATE);
        let end = (count - 1 - i) as f64 / f64::from(SAMPLE_RATE);
        let x = voice(t) * (t / 0.008).min(1.0) * (end / 0.025).min(1.0);
        for (channel, gain) in out[(start + i) % frames].iter_mut().zip(gains) {
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
pub fn mix<const N: usize>(stems: &[Pcm; N]) -> Pcm {
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

pub const DANCER_STEMS: [&str; 8] = [
    "drums", "bass", "pads", "arp", "stabs", "lead", "choir", "surge",
];
pub const DANCER_BPM: f64 = 132.0;
pub const DANCER_FRAMES: usize = (44100.0 * 60.0 / DANCER_BPM * 160.0 + 0.5) as usize;

/// Forty bars: invitation, chase, flight, collapse, and a dominant turnaround.
/// Phase layers share a conservative absolute-sum gain, so every subset has headroom.
pub fn render_dancer() -> [Pcm; 8] {
    let mut stems: [Signal; 8] = std::array::from_fn(|_| vec![[0.0; 2]; DANCER_FRAMES]);
    let beat = 60.0 / DANCER_BPM;
    let chords = [
        (38, [62, 65, 69, 76]),
        (34, [62, 65, 69, 70]),
        (41, [60, 65, 69, 72]),
        (33, [61, 64, 67, 70]),
    ];
    let mut noise = Noise(0xda4c_e123);
    for bar in 0..40 {
        let section = bar / 8;
        let (root, chord) = chords[(bar / 2) % 4];
        let at = |b: f64| (bar as f64 * 4.0 + b) * beat;
        for b in 0..4 {
            event_seconds(&mut stems[0], at(b as f64), 0.32, 0.0, |t| {
                0.48 * (TAU * (48.0 * t + 1.5 * (1.0 - (-t / 0.022).exp()))).sin()
                    * (-t / 0.085).exp()
            });
            if b % 2 == 1 {
                event_seconds(&mut stems[0], at(b as f64), 0.19, -0.1, |t| {
                    0.15 * (noise.next() + 0.4 * (TAU * 185.0 * t).sin()) * (-t / 0.045).exp()
                });
            }
            for off in [0.0, 0.5] {
                let note = root + if off == 0.0 { 0 } else { 12 };
                event_seconds(&mut stems[1], at(b as f64 + off), beat * 0.46, 0.0, |t| {
                    let p = TAU * hz(note) * t;
                    0.26 * (p.sin() + 0.32 * (2.0 * p).sin() + 0.12 * (3.0 * p).sin())
                        * (-t / 0.18).exp()
                });
                event_seconds(&mut stems[0], at(b as f64 + off), 0.09, 0.35, |t| {
                    0.06 * noise.next() * (-t / 0.018).exp()
                });
            }
            // Offbeat kick/toms and sixteenths make the final phase run twice as fast.
            event_seconds(&mut stems[7], at(b as f64 + 0.5), 0.26, -0.2, |t| {
                let f = if b == 3 { 110.0 } else { 65.0 };
                0.26 * (TAU * (f * t + 0.6 * (1.0 - (-t / 0.04).exp()))).sin() * (-t / 0.09).exp()
            });
            for off in [0.25, 0.75] {
                event_seconds(&mut stems[7], at(b as f64 + off), 0.07, -0.4, |t| {
                    0.045 * noise.next() * (-t / 0.014).exp()
                });
            }
        }
        for (v, note) in chord.into_iter().enumerate() {
            let pan = (v as f64 - 1.5) * 0.4;
            event_seconds(&mut stems[2], at(0.0), beat * 5.0, pan, |t| {
                let p = TAU * hz(note - 12) * t;
                let env = (t / 0.16).min(1.0) * ((beat * 5.0 - t) / 0.6).min(1.0);
                0.055 * env * (p.sin() + 0.4 * (p * 1.003).sin())
            });
            event_seconds(&mut stems[6], at(0.0), beat * 5.5, -pan, |t| {
                let p = TAU * hz(note) * t + 0.14 * (TAU * 4.8 * t).sin();
                let env = (t / 0.24).min(1.0) * ((beat * 5.5 - t) / 0.7).min(1.0);
                0.036
                    * env
                    * (p.sin()
                        + 0.5 * (p * 2.0).sin()
                        + 0.3 * (p * 3.0).sin()
                        + 0.12 * (p * 6.0).sin())
            });
            for off in [0.75, 1.5, 2.75, 3.5] {
                event_seconds(&mut stems[4], at(off), 0.14, pan * 0.7, |t| {
                    let p = TAU * hz(note) * t;
                    0.065
                        * (p.sin() + 0.45 * (2.0 * p).sin() + 0.2 * (3.0 * p).sin())
                        * (-t / 0.065).exp()
                });
            }
        }
        for step in 0..16 {
            let note = chord[(step + section) % 4] + if step % 8 >= 4 { 12 } else { 0 };
            event_seconds(
                &mut stems[3],
                at(step as f64 / 4.0),
                0.18,
                if step % 2 == 0 { -0.55 } else { 0.55 },
                |t| {
                    let p = TAU * hz(note) * t;
                    0.082
                        * (p + 0.8 * (-t / 0.06).exp() * (2.0 * p).sin()).sin()
                        * (-t / 0.07).exp()
                },
            );
        }
        // Toast stretched into a rising cry, then a falling answer on the dominant.
        let melody = if bar % 8 >= 6 {
            [81, 79, 76, 73]
        } else if section == 2 {
            [86, 89, 88, 81]
        } else {
            [74, 77, 76, 81]
        };
        for (i, note) in melody.into_iter().enumerate() {
            let duration = if i == 3 { beat * 1.8 } else { beat * 0.95 };
            event_seconds(&mut stems[5], at(i as f64), duration, 0.12, |t| {
                let p = TAU * hz(note) * t
                    + 0.19 * (TAU * 5.2 * t).sin()
                    + 0.08 * (TAU * 0.7 * t).sin();
                let env = (t / 0.045).min(1.0) * ((duration - t) / 0.16).min(1.0);
                0.12 * env * (p.sin() + 0.35 * (p * 1.004).sin() + 0.22 * (2.0 * p).sin())
            });
        }
        if bar % 8 == 7 {
            for step in 0..8 {
                event_seconds(
                    &mut stems[0],
                    at(2.0 + step as f64 / 4.0),
                    0.38,
                    (step as f64 - 3.5) * 0.12,
                    |t| {
                        0.16 * (TAU
                            * ((150.0 - step as f64 * 9.0) * t + 0.7 * (1.0 - (-t / 0.035).exp())))
                        .sin()
                            * (-t / 0.12).exp()
                    },
                );
            }
        }
    }
    finish(
        stems,
        [
            8500.0, 1100.0, 3400.0, 6800.0, 4800.0, 6500.0, 4200.0, 7500.0,
        ],
        [20.0, 20.0, 190.0, 330.0, 260.0, 380.0, 240.0, 25.0],
        [0.06, 0.02, 0.22, 0.16, 0.12, 0.24, 0.3, 0.08],
    )
}

/// Band-limits each stem between its high and low cutoffs, adds its room, and scales all of them
/// by one gain that keeps the absolute sum at -6 dBFS, so every subset of layers has headroom.
fn finish<const N: usize>(
    mut stems: [Signal; N],
    high: [f64; N],
    low: [f64; N],
    wet: [f64; N],
) -> [Pcm; N] {
    for (i, data) in stems.iter_mut().enumerate() {
        low_pass(data, high[i]);
        let mut lows = data.clone();
        low_pass(&mut lows, low[i]);
        for (frame, low) in data.iter_mut().zip(lows) {
            for c in 0..2 {
                frame[c] -= low[c];
            }
        }
        room(data, wet[i]);
    }
    let frames = stems[0].len();
    let peak = (0..frames)
        .flat_map(|i| (0..2).map(move |c| (i, c)))
        .map(|(i, c)| stems.iter().map(|s| s[i][c].abs()).sum::<f64>())
        .fold(0.0_f64, f64::max);
    let gain = 10.0_f64.powf(-6.0 / 20.0) / peak;
    stems.map(|s| {
        s.into_iter()
            .map(|f| f.map(|v| (v * gain * f64::from(i16::MAX)).round() as i16))
            .collect()
    })
}

pub const DOORMAN_STEMS: [&str; 8] = [
    "drums", "bass", "bells", "organ", "stabs", "march", "lead", "clock",
];
/// How many of the Doorman's stems play in each of his phases: the list, impatience, closing time.
pub const DOORMAN_LAYERS: [usize; 3] = [3, 6, 8];
pub const DOORMAN_BPM: f64 = 104.0;
pub const DOORMAN_FRAMES: usize = (44100.0 * 60.0 / DOORMAN_BPM * 128.0 + 0.5) as usize;

/// A tubular bell: inharmonic partials over a long decay, struck.
fn bell(note: u8, t: f64, decay: f64) -> f64 {
    let f = hz(note);
    [
        (1.0, 1.0),
        (2.76, 0.5),
        (5.40, 0.25),
        (8.93, 0.12),
        (0.5, 0.3),
    ]
    .into_iter()
    .map(|(ratio, gain)| gain * (TAU * f * ratio * t).sin() * (-t * ratio.sqrt() / decay).exp())
    .sum()
}

/// A brassy saw from its first eight harmonics, with a little swell.
fn brass(note: u8, t: f64, bright: f64) -> f64 {
    let f = hz(note);
    let swell = 1.0 - (-t / 0.03).exp();
    (1..=8)
        .map(|n| {
            let n = f64::from(n);
            (TAU * f * n * t + 0.05 * (TAU * 5.5 * t).sin()).sin() / n * bright.powf(n - 1.0)
        })
        .sum::<f64>()
        * swell
}

/// Thirty-two bars of a slow processional in C minor, for the man at the door.
/// The list is timpani, the three knocks of his staff, a low bass and a door chime;
/// impatience brings a harmonium, brass and a marching snare; closing time a mocking
/// fanfare over a ticking clock, and a hand bell calling last orders.
pub fn render_doorman() -> [Pcm; 8] {
    let mut stems: [Signal; 8] = std::array::from_fn(|_| vec![[0.0; 2]; DOORMAN_FRAMES]);
    let beat = 60.0 / DOORMAN_BPM;
    // i, VI, iv, V: two bars each.
    let chords: [(u8, [u8; 4]); 4] = [
        (36, [60, 63, 67, 72]),
        (32, [60, 63, 68, 72]),
        (41, [60, 65, 68, 72]),
        (43, [59, 62, 67, 71]),
    ];
    // The fanfare over each two bars: beat, length, note.
    let fanfare: [[(f64, f64, u8); 6]; 4] = [
        [
            (0.0, 1.5, 67),
            (1.5, 0.5, 72),
            (2.0, 2.0, 75),
            (4.0, 1.0, 74),
            (5.0, 1.0, 72),
            (6.0, 2.0, 67),
        ],
        [
            (0.0, 1.5, 68),
            (1.5, 0.5, 72),
            (2.0, 2.0, 75),
            (4.0, 1.0, 77),
            (5.0, 1.0, 75),
            (6.0, 2.0, 72),
        ],
        [
            (0.0, 1.5, 65),
            (1.5, 0.5, 68),
            (2.0, 2.0, 72),
            (4.0, 1.0, 77),
            (5.0, 1.0, 75),
            (6.0, 2.0, 74),
        ],
        [
            (0.0, 1.5, 71),
            (1.5, 0.5, 74),
            (2.0, 2.0, 77),
            (4.0, 1.0, 75),
            (5.0, 1.0, 74),
            (6.0, 2.0, 71),
        ],
    ];
    let mut noise = Noise(0x0d00_4a11);
    for pair in 0..16 {
        let section = pair / 4;
        let (root, chord) = chords[pair % 4];
        let at = |b: f64| (pair as f64 * 8.0 + b) * beat;

        // Drums: a timpani on the root and fifth on each bar's downbeat and third beat,
        // a deep bass drum under it, and in the second bar the staff's three knocks.
        for bar in 0..2 {
            for (b, note) in [(0.0, root + 12), (2.0, root + 19)] {
                let b = bar as f64 * 4.0 + b;
                event_seconds(&mut stems[0], at(b), 1.2, -0.15, |t| {
                    let f = hz(note) * (1.0 + 0.12 * (-t / 0.03).exp());
                    0.42 * ((TAU * f * t).sin() + 0.3 * (TAU * f * 1.5 * t).sin())
                        * (-t / 0.38).exp()
                });
                event_seconds(&mut stems[0], at(b), 0.5, 0.0, |t| {
                    0.4 * (TAU * (42.0 * t + 2.0 * (1.0 - (-t / 0.03).exp()))).sin()
                        * (-t / 0.16).exp()
                });
            }
        }
        for b in [6.0, 6.5, 7.0] {
            event_seconds(&mut stems[0], at(b), 0.3, 0.2, |t| {
                0.3 * (0.6 * noise.next() * (-t / 0.012).exp()
                    + (TAU * 95.0 * t).sin() * (-t / 0.07).exp())
            });
        }

        // Bass: a dotted march on the root, an octave leap on the pickup.
        for bar in 0..2 {
            for (b, len, up) in [(0.0, 1.4, 0), (1.5, 0.45, 12), (2.0, 1.9, 0)] {
                let note = root + up;
                let len = len * beat;
                event_seconds(&mut stems[1], at(bar as f64 * 4.0 + b), len, 0.0, |t| {
                    let p = TAU * hz(note) * t;
                    let env = (t / 0.02).min(1.0) * ((len - t) / 0.08).clamp(0.0, 1.0);
                    0.3 * env * (p.sin() + 0.45 * (2.0 * p).sin() + 0.2 * (3.0 * p).sin())
                });
            }
        }

        // Bells: the door chime, falling from the chord's top to its root,
        // and in the later sections answered from below.
        let chime: &[(f64, u8)] = if section == 0 {
            &[(0.0, chord[3]), (1.0, chord[1])]
        } else {
            &[
                (0.0, chord[3]),
                (1.0, chord[1]),
                (4.0, chord[2]),
                (5.0, chord[0]),
            ]
        };
        for &(b, note) in chime {
            let pan = if b < 2.0 { -0.45 } else { 0.45 };
            event_seconds(&mut stems[2], at(b), 3.2, pan, |t| 0.2 * bell(note, t, 1.1));
        }

        // Organ: the chord held in a pompous dotted rhythm, with a slow tremolo.
        for bar in 0..2 {
            for (b, len) in [(0.0, 1.45), (1.5, 0.45), (2.0, 1.95)] {
                let len = len * beat;
                for (v, note) in chord.into_iter().enumerate() {
                    let pan = (v as f64 - 1.5) * 0.35;
                    event_seconds(&mut stems[3], at(bar as f64 * 4.0 + b), len, pan, |t| {
                        let p = TAU * hz(note - 12) * t;
                        let env = (t / 0.03).min(1.0) * ((len - t) / 0.06).clamp(0.0, 1.0);
                        let trem = 1.0 + 0.15 * (TAU * 5.0 * t).sin();
                        0.045
                            * env
                            * trem
                            * (p.sin()
                                + 0.5 * (2.0 * p).sin()
                                + 0.3 * (3.0 * p).sin()
                                + 0.15 * (4.0 * p).sin())
                    });
                }
            }
        }

        // Stabs: brass on the offbeats, pushing into each new chord.
        for b in [1.5, 3.5, 5.5, 7.0, 7.5] {
            for (v, note) in chord.into_iter().enumerate().skip(1) {
                let pan = (v as f64 - 2.0) * 0.5;
                event_seconds(&mut stems[4], at(b), 0.22, pan, |t| {
                    0.17 * brass(note - 12, t, 0.75) * (-t / 0.14).exp()
                });
            }
        }

        // March: a snare on two and four, and a roll into each pair's last beat.
        for b in [1.0, 3.0, 5.0] {
            event_seconds(&mut stems[5], at(b), 0.2, 0.1, |t| {
                0.36 * (noise.next() + 0.5 * (TAU * 190.0 * t).sin()) * (-t / 0.05).exp()
            });
        }
        for step in 0..8 {
            let gain = 0.1 + 0.03 * step as f64;
            event_seconds(&mut stems[5], at(6.0 + step as f64 / 4.0), 0.12, 0.1, |t| {
                gain * (noise.next() + 0.4 * (TAU * 200.0 * t).sin()) * (-t / 0.035).exp()
            });
        }

        // Lead: the fanfare, an octave up in the third section.
        let lift = if section == 2 { 12 } else { 0 };
        for (b, len, note) in fanfare[pair % 4] {
            let note = note + lift;
            let len = len * beat;
            event_seconds(&mut stems[6], at(b), len, -0.1, |t| {
                let vibrato = 0.012 * (TAU * 5.8 * t).sin() * (t / 0.3).min(1.0);
                let p = TAU * hz(note) * t * (1.0 + vibrato);
                let env = (t / 0.025).min(1.0) * ((len - t) / 0.1).clamp(0.0, 1.0);
                0.14 * env
                    * (p.sin()
                        + 0.6 * (2.0 * p).sin()
                        + 0.45 * (3.0 * p).sin()
                        + 0.25 * (4.0 * p).sin()
                        + 0.12 * (5.0 * p).sin())
            });
        }

        // Clock: tick and tock on every eighth, and at the end of each section
        // a hand bell rung fast for last orders.
        for step in 0..16 {
            let f = if step % 2 == 0 { 1900.0 } else { 1450.0 };
            event_seconds(&mut stems[7], at(step as f64 / 2.0), 0.05, 0.5, |t| {
                0.18 * (TAU * f * t).sin() * (-t / 0.008).exp()
            });
        }
        if pair % 4 == 3 {
            for step in 0..12 {
                let note = if step % 2 == 0 { 96 } else { 91 };
                event_seconds(&mut stems[7], at(5.0 + step as f64 / 4.0), 0.6, -0.5, |t| {
                    0.06 * bell(note, t, 0.25)
                });
            }
        }
    }
    finish(
        stems,
        [
            5000.0, 900.0, 7000.0, 3200.0, 2600.0, 7500.0, 4200.0, 9000.0,
        ],
        [25.0, 25.0, 180.0, 120.0, 110.0, 140.0, 260.0, 600.0],
        [0.1, 0.02, 0.35, 0.2, 0.14, 0.12, 0.22, 0.18],
    )
}

pub const FEAST_STEMS: [&str; 5] = STEMS;

/// The feast: 32 bars of harpsichord over a disco rhythm section in G minor, at the ballroom's
/// tempo so the rooms' lights keep the beat. A baroque circle of fifths turns under a
/// four-on-the-floor; cutlery and too many voices fill the room around it.
pub fn render_feast() -> [Pcm; 5] {
    let mut stems: [Signal; 5] = std::array::from_fn(|_| vec![[0.0; 2]; FRAMES]);
    // i, iv, VII, III, VI, ii half-diminished, V7, i: a bar each.
    let chords: [(u8, [u8; 3]); 8] = [
        (43, [62, 67, 70]),
        (48, [60, 63, 67]),
        (41, [60, 65, 69]),
        (46, [62, 65, 70]),
        (39, [63, 67, 70]),
        (45, [60, 63, 69]),
        (50, [62, 66, 69]),
        (43, [62, 67, 70]),
    ];
    let mut noise = Noise(0xfea5_7001);
    for bar in 0..BARS {
        let section = bar / 8;
        let (root, chord) = chords[bar % 8];
        let at = |b: f64| bar as f64 * 4.0 + b;

        // Drums: four on the floor, a clap on two and four, open hats on the offbeats.
        for b in 0..4 {
            event(&mut stems[0], at(b as f64), 0.3, 0.0, |t| {
                0.45 * (TAU * (50.0 * t + 1.6 * (1.0 - (-t / 0.02).exp()))).sin()
                    * (-t / 0.12).exp()
            });
            if b % 2 == 1 {
                let spread = [noise.next() * 0.004, noise.next() * 0.004];
                for (i, d) in spread.into_iter().enumerate() {
                    event_seconds(
                        &mut stems[0],
                        at(b as f64) * BEAT + 0.008 * i as f64 + d.abs(),
                        0.16,
                        0.15,
                        |t| 0.12 * noise.next() * (-t / 0.03).exp(),
                    );
                }
            }
            event(&mut stems[0], at(b as f64 + 0.5), 0.2, 0.3, |t| {
                0.05 * noise.next() * (-t / 0.06).exp()
            });
        }

        // Bass: octaves in eighths, the disco bounce.
        for step in 0..8 {
            let note = root + if step % 2 == 0 { 0 } else { 12 };
            event(&mut stems[1], at(step as f64 / 2.0), 0.22, 0.0, |t| {
                let p = TAU * hz(note) * t;
                0.28 * (p.sin() + 0.35 * (2.0 * p).sin() + 0.15 * (3.0 * p).sin())
                    * (-t / 0.16).exp()
            });
        }

        // Harmony: the harpsichord's continuo, a broken chord in sixteenths.
        for step in 0..16 {
            let note = chord[[0, 1, 2, 1][step % 4]] - 12 + if step >= 8 { 12 } else { 0 };
            let pan = if step % 2 == 0 { -0.35 } else { 0.35 };
            event(&mut stems[2], at(step as f64 / 4.0), 0.5, pan, |t| {
                0.085 * pluck(note, t)
            });
        }

        // Lead: a running figure through the chord, an octave up the second time round.
        let tones: Vec<u8> = (0..3)
            .flat_map(|o| chord.iter().map(move |n| n + 12 * o))
            .collect();
        let figure: [usize; 8] = if section % 2 == 0 {
            [3, 2, 1, 2, 4, 3, 2, 3]
        } else {
            [5, 4, 3, 4, 3, 2, 1, 0]
        };
        let lift = if section >= 2 { 12 } else { 0 };
        for (i, idx) in figure.into_iter().enumerate() {
            let note = tones[idx] + lift;
            event(&mut stems[3], at(i as f64 / 2.0), 0.45, 0.1, |t| {
                0.12 * pluck(note, t)
            });
        }

        // Texture: forks on plates, and voices talking over each other.
        for step in 0..16 {
            if noise.next() > 0.0 {
                continue;
            }
            let pan = noise.next() * 0.9;
            let pitch = 1.0 + 0.3 * noise.next();
            event(&mut stems[4], at(step as f64 / 4.0), 0.12, pan, |t| {
                0.04 * [(2100.0, 1.0), (3370.0, 0.6), (5230.0, 0.4)]
                    .into_iter()
                    .map(|(f, g)| g * (TAU * f * pitch * t).sin())
                    .sum::<f64>()
                    * (-t / 0.03).exp()
            });
        }
        for step in 0..8 {
            if noise.next() > -0.2 {
                continue;
            }
            let pan = noise.next() * 0.9;
            let pitch = 140.0 + 80.0 * (noise.next() + 1.0);
            let vowel = 500.0 + 450.0 * (noise.next() + 1.0);
            let len = 0.14 + 0.1 * (noise.next() + 1.0);
            event(&mut stems[4], at(step as f64 / 2.0), len, pan, |t| {
                let env = (t / 0.02).min(1.0) * ((len - t) / 0.04).clamp(0.0, 1.0);
                0.035
                    * env
                    * (1..=8)
                        .map(|n| {
                            let h = pitch * (1.0 - 0.1 * t / len) * f64::from(n);
                            let formant = (-((h - vowel) / 300.0).powi(2)).exp() + 0.15;
                            formant * (TAU * h * t).sin()
                        })
                        .sum::<f64>()
            });
        }
    }
    finish(
        stems,
        [6500.0, 1000.0, 7000.0, 7500.0, 8000.0],
        [25.0, 25.0, 140.0, 220.0, 160.0],
        [0.06, 0.02, 0.18, 0.2, 0.3],
    )
}

pub const GLUTTON_STEMS: [&str; 8] = [
    "drums",
    "bass",
    "harpsichord",
    "cutlery",
    "brass",
    "voices",
    "organ",
    "belly",
];
/// How many of the Glutton's stems play in each of his phases: appetite, seconds, too much.
pub const GLUTTON_LAYERS: [usize; 3] = [3, 6, 8];
pub const GLUTTON_BPM: f64 = 144.0;
pub const GLUTTON_FRAMES: usize = (44100.0 * 60.0 / GLUTTON_BPM * 128.0 + 0.5) as usize;

/// A harpsichord's pluck: bright, odd-heavy and quick to fade.
fn pluck(note: u8, t: f64) -> f64 {
    let f = hz(note);
    (1..=7)
        .map(|n| {
            let n = f64::from(n);
            let odd = if n as u32 % 2 == 1 { 1.0 } else { 0.6 };
            odd * (TAU * f * n * t).sin() / n * (-t * (2.5 + n) / 0.9).exp()
        })
        .sum()
}

/// Thirty-two bars of a galop in A minor, for the guest who cannot stop.
/// Appetite is an oom-pah tuba, galloping drums and a harpsichord; seconds brings cutlery,
/// a brass tune with a can-can's leer and a babble of voices; too much an organ that will not
/// let go and a low churning in the belly, with a sour cluster every other bar.
pub fn render_glutton() -> [Pcm; 8] {
    let mut stems: [Signal; 8] = std::array::from_fn(|_| vec![[0.0; 2]; GLUTTON_FRAMES]);
    let beat = 60.0 / GLUTTON_BPM;
    let am = (45, [57, 60, 64]);
    let dm = (50, [57, 62, 65]);
    let e7 = (40, [56, 59, 62]);
    let f = (41, [57, 60, 65]);
    let chords = [am, am, dm, dm, e7, e7, am, am, f, f, dm, dm, e7, e7, am, e7];
    // The tune, a bar at a time: beat, length, note.
    let tune: [&[(f64, f64, u8)]; 8] = [
        &[
            (0.0, 0.5, 69),
            (0.5, 0.5, 72),
            (1.0, 0.5, 76),
            (1.5, 0.5, 72),
            (2.0, 1.0, 81),
            (3.0, 1.0, 76),
        ],
        &[
            (0.0, 0.5, 77),
            (0.5, 0.5, 76),
            (1.0, 0.5, 74),
            (1.5, 0.5, 72),
            (2.0, 2.0, 76),
        ],
        &[
            (0.0, 0.5, 74),
            (0.5, 0.5, 77),
            (1.0, 0.5, 81),
            (1.5, 0.5, 77),
            (2.0, 1.0, 86),
            (3.0, 1.0, 81),
        ],
        &[
            (0.0, 0.5, 79),
            (0.5, 0.5, 77),
            (1.0, 0.5, 76),
            (1.5, 0.5, 74),
            (2.0, 2.0, 77),
        ],
        &[
            (0.0, 0.5, 76),
            (0.5, 0.5, 80),
            (1.0, 0.5, 83),
            (1.5, 0.5, 80),
            (2.0, 1.0, 88),
            (3.0, 1.0, 83),
        ],
        &[
            (0.0, 0.5, 86),
            (0.5, 0.5, 84),
            (1.0, 0.5, 83),
            (1.5, 0.5, 80),
            (2.0, 2.0, 83),
        ],
        &[
            (0.0, 0.5, 81),
            (0.5, 0.5, 76),
            (1.0, 0.5, 72),
            (1.5, 0.5, 69),
            (2.0, 1.0, 76),
            (3.0, 1.0, 72),
        ],
        &[(0.0, 1.0, 69), (1.0, 1.0, 64), (2.0, 2.0, 57)],
    ];
    let mut noise = Noise(0x6107_7015);
    for bar in 0..32 {
        let (root, chord) = chords[bar % 16];
        let at = |b: f64| (bar as f64 * 4.0 + b) * beat;

        // Drums: a galop. Kick on one and three, snare on two and four, hats on every eighth.
        for b in 0..4 {
            if b % 2 == 0 {
                event_seconds(&mut stems[0], at(b as f64), 0.3, 0.0, |t| {
                    0.42 * (TAU * (52.0 * t + 1.8 * (1.0 - (-t / 0.025).exp()))).sin()
                        * (-t / 0.11).exp()
                });
            } else {
                event_seconds(&mut stems[0], at(b as f64), 0.2, -0.1, |t| {
                    0.2 * (noise.next() + 0.5 * (TAU * 200.0 * t).sin()) * (-t / 0.06).exp()
                });
            }
            for half in [0.0, 0.5] {
                event_seconds(&mut stems[0], at(b as f64 + half), 0.06, 0.35, |t| {
                    0.05 * noise.next() * (-t / 0.012).exp()
                });
            }
        }

        // Bass: a tuba's oom on one and three, root then fifth.
        for (b, note) in [(0.0, root), (2.0, root + 7)] {
            let len = beat * 0.7;
            event_seconds(&mut stems[1], at(b), len, 0.0, |t| {
                let env = (t / 0.02).min(1.0) * ((len - t) / 0.05).clamp(0.0, 1.0);
                0.3 * env * brass(note, t, 0.55)
            });
        }

        // Harpsichord: the pah on two and four, and a broken chord running in eighths.
        for b in [1.0, 3.0] {
            for (v, note) in chord.into_iter().enumerate() {
                event_seconds(
                    &mut stems[2],
                    at(b + v as f64 * 0.012),
                    0.6,
                    (v as f64 - 1.0) * 0.3,
                    |t| 0.08 * pluck(note, t),
                );
            }
        }
        for step in 0..8 {
            let note = chord[[0, 1, 2, 1, 0, 2, 1, 2][step]] + 12;
            event_seconds(&mut stems[2], at(step as f64 / 2.0), 0.4, -0.4, |t| {
                0.05 * pluck(note, t)
            });
        }

        // Cutlery: forks on plates, never quite in time.
        for step in 0..16 {
            if noise.next() > 0.25 {
                continue;
            }
            let pan = noise.next() * 0.8;
            let pitch = 1.0 + 0.25 * noise.next();
            let late = 0.02 * (noise.next() + 1.0);
            event_seconds(
                &mut stems[3],
                at(step as f64 / 4.0) + late,
                0.12,
                pan,
                |t| {
                    0.07 * [(2100.0, 1.0), (3370.0, 0.6), (5230.0, 0.4)]
                        .into_iter()
                        .map(|(f, g)| g * (TAU * f * pitch * t).sin())
                        .sum::<f64>()
                        * (-t / 0.035).exp()
                },
            );
        }

        // Brass: the tune, the second time through an octave down and leering.
        let drop = if bar % 16 >= 8 { 12 } else { 0 };
        for &(b, len, note) in tune[bar % 8] {
            let note = note - drop;
            let len = len * beat * 0.92;
            event_seconds(&mut stems[4], at(b), len, 0.15, |t| {
                let env = (t / 0.02).min(1.0) * ((len - t) / 0.05).clamp(0.0, 1.0);
                0.1 * env * brass(note, t, 0.8)
            });
        }

        // Voices: too many of them, talking over each other with their mouths full.
        for step in 0..8 {
            if noise.next() > 0.2 {
                continue;
            }
            let pan = noise.next() * 0.9;
            let pitch = 150.0 + 70.0 * (noise.next() + 1.0);
            let vowel = 500.0 + 450.0 * (noise.next() + 1.0);
            let len = 0.12 + 0.08 * (noise.next() + 1.0);
            event_seconds(&mut stems[5], at(step as f64 / 2.0), len, pan, |t| {
                let glide = 1.0 - 0.15 * t / len;
                let env = (t / 0.015).min(1.0) * ((len - t) / 0.03).clamp(0.0, 1.0);
                0.05 * env
                    * (1..=8)
                        .map(|n| {
                            let h = pitch * glide * f64::from(n);
                            let formant = (-((h - vowel) / 300.0).powi(2)).exp() + 0.15;
                            formant * (TAU * h * t).sin()
                        })
                        .sum::<f64>()
            });
        }

        // Organ: the chord held a whole bar, swelling.
        let len = beat * 4.0;
        for (v, note) in chord.into_iter().enumerate() {
            event_seconds(&mut stems[6], at(0.0), len, (v as f64 - 1.0) * 0.4, |t| {
                let p = TAU * hz(note) * t;
                let env = (t / 0.4).min(1.0) * ((len - t) / 0.1).clamp(0.0, 1.0);
                0.05 * env
                    * (p.sin()
                        + 0.5 * (2.0 * p).sin()
                        + 0.35 * (3.0 * p).sin()
                        + 0.2 * (4.0 * p).sin())
            });
        }

        // Belly: a low churn under everything, and every other bar a sour cluster.
        event_seconds(&mut stems[7], at(0.0), len, 0.0, |t| {
            let wobble = 1.0 + 0.04 * (TAU * 1.7 * t).sin();
            0.2 * (TAU * 41.0 * wobble * t).sin() * (0.6 + 0.4 * (TAU * 0.9 * t).sin())
        });
        if bar % 2 == 1 {
            for (v, note) in [root + 12, root + 13, root + 18].into_iter().enumerate() {
                event_seconds(
                    &mut stems[7],
                    at(3.0),
                    beat * 1.2,
                    (v as f64 - 1.0) * 0.5,
                    |t| 0.06 * brass(note, t, 0.7) * (-t / 0.3).exp(),
                );
            }
        }
    }
    finish(
        stems,
        [
            6000.0, 900.0, 7500.0, 9000.0, 4500.0, 3200.0, 3000.0, 1600.0,
        ],
        [30.0, 25.0, 150.0, 1200.0, 200.0, 140.0, 110.0, 25.0],
        [0.06, 0.02, 0.18, 0.25, 0.16, 0.3, 0.22, 0.1],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dancer_layers_have_headroom_and_wrapped_tails() {
        let stems = render_dancer();
        for pcm in stems.iter().chain(std::iter::once(&mix(&stems))) {
            assert_eq!(pcm.len(), DANCER_FRAMES);
            assert!(levels(pcm).1 <= -5.99);
            for (c, _) in pcm[0].iter().enumerate() {
                let seam = (i32::from(pcm[0][c]) - i32::from(pcm[DANCER_FRAMES - 1][c])).abs();
                // A bright summed waveform can slope faster than the ballroom.
                // Reject a boundary spike relative to its surrounding 10 ms.
                let nearby = pcm[..441]
                    .windows(2)
                    .chain(pcm[pcm.len() - 441..].windows(2))
                    .map(|w| (i32::from(w[1][c]) - i32::from(w[0][c])).abs())
                    .max()
                    .unwrap();
                assert!(
                    seam < 300 && seam <= nearby + 2,
                    "dancer seam {seam}, nearby slope {nearby}"
                );
            }
        }
    }

    #[test]
    fn doorman_layers_have_headroom_and_wrapped_tails() {
        let stems = render_doorman();
        for count in DOORMAN_LAYERS {
            let layers: [Pcm; 8] = std::array::from_fn(|i| {
                if i < count {
                    stems[i].clone()
                } else {
                    vec![[0; 2]; DOORMAN_FRAMES]
                }
            });
            assert!(levels(&mix(&layers)).1 <= -5.99);
        }
        for pcm in &stems {
            assert_eq!(pcm.len(), DOORMAN_FRAMES);
            for c in 0..2 {
                let seam = (i32::from(pcm[0][c]) - i32::from(pcm[DOORMAN_FRAMES - 1][c])).abs();
                let nearby = pcm[..441]
                    .windows(2)
                    .chain(pcm[pcm.len() - 441..].windows(2))
                    .map(|w| (i32::from(w[1][c]) - i32::from(w[0][c])).abs())
                    .max()
                    .unwrap();
                assert!(
                    seam <= nearby + 2,
                    "doorman seam {seam}, nearby slope {nearby}"
                );
            }
        }
    }

    #[test]
    fn glutton_layers_have_headroom() {
        let stems = render_glutton();
        for count in GLUTTON_LAYERS {
            let layers: [Pcm; 8] = std::array::from_fn(|i| {
                if i < count {
                    stems[i].clone()
                } else {
                    vec![[0; 2]; GLUTTON_FRAMES]
                }
            });
            assert!(levels(&mix(&layers)).1 <= -5.99);
        }
        for pcm in &stems {
            assert_eq!(pcm.len(), GLUTTON_FRAMES);
            for c in 0..2 {
                let seam = (i32::from(pcm[0][c]) - i32::from(pcm[GLUTTON_FRAMES - 1][c])).abs();
                let nearby = pcm[..441]
                    .windows(2)
                    .chain(pcm[pcm.len() - 441..].windows(2))
                    .map(|w| (i32::from(w[1][c]) - i32::from(w[0][c])).abs())
                    .max()
                    .unwrap();
                assert!(
                    seam <= nearby + 2,
                    "glutton seam {seam}, nearby slope {nearby}"
                );
            }
        }
    }

    #[test]
    fn the_feast_loops_with_headroom() {
        let stems = render_feast();
        assert!(levels(&mix(&stems)).1 <= -5.99);
        for pcm in &stems {
            assert_eq!(pcm.len(), FRAMES);
            for c in 0..2 {
                let seam = (i32::from(pcm[0][c]) - i32::from(pcm[FRAMES - 1][c])).abs();
                let nearby = pcm[..441]
                    .windows(2)
                    .chain(pcm[pcm.len() - 441..].windows(2))
                    .map(|w| (i32::from(w[1][c]) - i32::from(w[0][c])).abs())
                    .max()
                    .unwrap();
                assert!(
                    seam <= nearby + 2,
                    "feast seam {seam}, nearby slope {nearby}"
                );
            }
        }
    }

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
