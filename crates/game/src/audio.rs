//! Rendered effects and a music bus, driven by Bevy messages.
//!
//! Send `PlaySfx::new(Sfx::HitHeavy)` and `Duck` separately at impact.
//! `Audio::music_track` accepts future music playback on the ducked bus.
use bevy::prelude::*;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
    sound::static_sound::StaticSoundData,
    track::{TrackBuilder, TrackHandle},
};
use std::{collections::HashMap, time::Duration};

macro_rules! sfx_catalog {
    ($($variant:ident => $name:literal,)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Sfx { $($variant,)* }
        impl Sfx {
            pub const ALL: &[Self] = &[$(Self::$variant,)*];
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)* }
            }
            #[cfg(any(target_arch = "wasm32", test))]
            fn bytes(self) -> &'static [u8] {
                match self { $(Self::$variant => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/sfx/", $name, ".wav")),)* }
            }
        }
    };
}
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/sfx/catalog.rs"
));

/// Plays an effect. Invalid or non-positive volume/rate values are ignored.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlaySfx {
    pub sound: Sfx,
    /// Linear gain: 1.0 preserves the rendered level.
    pub volume: f32,
    /// Playback rate: 1.0 preserves the rendered pitch and duration.
    pub pitch: f32,
    /// Disables the default ±3% pitch variation, useful for tuned cues.
    pub exact_pitch: bool,
}
impl PlaySfx {
    pub const fn new(sound: Sfx) -> Self {
        Self {
            sound,
            volume: 1.0,
            pitch: 1.0,
            exact_pitch: false,
        }
    }
    pub const fn exact(mut self) -> Self {
        self.exact_pitch = true;
        self
    }
}

/// Dips music to -6 dB for 150 ms, then recovers over 300 ms.
/// Repeated messages restart the hold without stacking attenuation.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct Duck;

pub struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_message::<Duck>()
            .add_systems(Startup, setup)
            .add_systems(Update, play);
    }
}

struct Output {
    manager: AudioManager<DefaultBackend>,
    music: TrackHandle,
}

/// Main-thread audio state. Use `NonSendMut<Audio>` to play future music on its bus.
/// A missing device leaves this resource available with no output.
pub struct Audio {
    output: Option<Output>,
    sounds: HashMap<Sfx, StaticSoundData>,
    rng: u32,
    duck: DuckHold,
    warned_playback: bool,
}
impl Audio {
    pub fn music_track(&mut self) -> Option<&mut TrackHandle> {
        self.output.as_mut().map(|output| &mut output.music)
    }
}

#[derive(Default)]
struct DuckHold(Option<f64>);
impl DuckHold {
    fn update(&mut self, now: f64, triggered: bool) -> Option<(f32, Duration)> {
        if triggered {
            self.0 = Some(now + 0.15);
            Some((-6.0, Duration::from_millis(5)))
        } else if self.0.is_some_and(|until| now >= until) {
            self.0 = None;
            Some((0.0, Duration::from_millis(300)))
        } else {
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load(sound: Sfx) -> Result<StaticSoundData, Box<dyn std::error::Error>> {
    let root = bevy::asset::io::file::FileAssetReader::get_base_path();
    let bytes = std::fs::read(
        root.join("assets/sfx")
            .join(format!("{}.wav", sound.name())),
    )?;
    Ok(StaticSoundData::from_cursor(std::io::Cursor::new(bytes))?)
}

#[cfg(target_arch = "wasm32")]
fn load(sound: Sfx) -> Result<StaticSoundData, Box<dyn std::error::Error>> {
    Ok(StaticSoundData::from_cursor(std::io::Cursor::new(
        sound.bytes(),
    ))?)
}

fn setup(world: &mut World) {
    let sounds = Sfx::ALL
        .iter()
        .filter_map(|&sound| match load(sound) {
            Ok(data) => Some((sound, data)),
            Err(error) => {
                warn!("Cannot load SFX {}: {error}", sound.name());
                None
            }
        })
        .collect();
    let output = match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
        Ok(mut manager) => match manager.add_sub_track(TrackBuilder::new()) {
            Ok(music) => Some(Output { manager, music }),
            Err(error) => {
                warn!("Audio disabled: cannot create music bus: {error}");
                None
            }
        },
        Err(error) => {
            warn!("Audio disabled: {error:?}");
            None
        }
    };
    world.insert_non_send(Audio {
        output,
        sounds,
        rng: 0x783a_912b,
        duck: DuckHold::default(),
        warned_playback: false,
    });
}

fn playback_parameters(message: &PlaySfx, rng: &mut u32) -> Option<(f32, f64)> {
    if !message.volume.is_finite()
        || message.volume <= 0.0
        || !message.pitch.is_finite()
        || message.pitch <= 0.0
    {
        return None;
    }
    let variation = if message.exact_pitch {
        1.0
    } else {
        *rng ^= *rng << 13;
        *rng ^= *rng >> 17;
        *rng ^= *rng << 5;
        0.97 + f64::from(*rng) / f64::from(u32::MAX) * 0.06
    };
    Some((
        20.0 * message.volume.log10(),
        f64::from(message.pitch) * variation,
    ))
}

fn play(
    mut audio: NonSendMut<Audio>,
    time: Res<Time<Real>>,
    mut requests: MessageReader<PlaySfx>,
    mut ducks: MessageReader<Duck>,
) {
    let triggered = ducks.read().count() > 0;
    if let Some((db, duration)) = audio.duck.update(time.elapsed_secs_f64(), triggered)
        && let Some(output) = &mut audio.output
    {
        output.music.set_volume(
            db,
            Tween {
                duration,
                ..default()
            },
        );
    }
    for request in requests.read() {
        let Some((db, rate)) = playback_parameters(request, &mut audio.rng) else {
            continue;
        };
        let Some(data) = audio.sounds.get(&request.sound) else {
            continue;
        };
        let data = data.volume(db).playback_rate(rate);
        if let Some(output) = &mut audio.output
            && let Err(error) = output.manager.play(data)
            && !audio.warned_playback
        {
            warn!("Cannot play SFX (further playback errors suppressed): {error}");
            audio.warned_playback = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_variant_has_a_decodable_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sfx");
        let files = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|f| f.path().extension().is_some_and(|e| e == "wav"))
            .count();
        assert_eq!(files, Sfx::ALL.len());
        for &sound in Sfx::ALL {
            let bytes = std::fs::read(dir.join(format!("{}.wav", sound.name()))).unwrap();
            assert_eq!(bytes, sound.bytes());
            StaticSoundData::from_cursor(std::io::Cursor::new(bytes)).unwrap();
        }
    }
    #[test]
    fn pitch_variation_exact_pitch_and_linear_gain() {
        let mut seed = 123;
        let mut request = PlaySfx::new(Sfx::Parry);
        request.volume = 0.5;
        request.pitch = 2.0;
        let mut rates = Vec::new();
        for _ in 0..100 {
            let (db, rate) = playback_parameters(&request, &mut seed).unwrap();
            assert!((db + 6.0206).abs() < 0.001);
            assert!((1.94..=2.06).contains(&rate));
            rates.push(rate);
        }
        assert!(rates.windows(2).any(|w| w[0] != w[1]));
        assert_eq!(
            playback_parameters(&request.exact(), &mut seed).unwrap().1,
            2.0
        );
        request.pitch = f32::NAN;
        assert!(playback_parameters(&request, &mut seed).is_none());
        request.pitch = 1.0;
        request.volume = 0.0;
        assert!(playback_parameters(&request, &mut seed).is_none());
    }
    #[test]
    fn duck_retriggers_and_recovers_once() {
        let mut duck = DuckHold::default();
        assert_eq!(duck.update(0.0, true).unwrap().0, -6.0);
        assert!(duck.update(0.10, false).is_none());
        assert_eq!(duck.update(0.14, true).unwrap().0, -6.0);
        assert!(duck.update(0.16, false).is_none());
        assert_eq!(
            duck.update(0.30, false),
            Some((0.0, Duration::from_millis(300)))
        );
        assert!(duck.update(0.7, false).is_none());
    }
    #[test]
    fn silent_output_consumes_messages_without_a_device() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PlaySfx>()
            .add_message::<Duck>()
            .insert_non_send(Audio {
                output: None,
                sounds: HashMap::new(),
                rng: 1,
                duck: DuckHold::default(),
                warned_playback: false,
            })
            .add_systems(Update, play);
        app.world_mut().write_message(PlaySfx::new(Sfx::HitHeavy));
        app.world_mut().write_message(Duck);
        app.update();
        assert!(app.world().non_send::<Audio>().output.is_none());
    }
}
