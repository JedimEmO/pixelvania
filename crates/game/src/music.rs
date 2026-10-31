//! Room-selected ballroom stems, mixed beneath the existing ducked music bus.
use bevy::prelude::*;

pub struct MusicPlugin;
impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.insert_non_send(native::Music::default())
            .add_systems(PostUpdate, native::update);
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, || info!("Ballroom music is disabled on wasm"));
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use crate::{audio::Audio, combat::Mood, rooms::ActiveRoom};
    use kira::{
        Decibels, Tween,
        effect::{
            filter::{FilterBuilder, FilterHandle},
            reverb::{ReverbBuilder, ReverbHandle},
        },
        sound::static_sound::{StaticSoundData, StaticSoundHandle},
        track::{TrackBuilder, TrackHandle, TrackPlaybackState},
    };
    use std::{error::Error, time::Duration};

    const STEMS: [&str; 5] = ["drums", "bass", "harmony", "lead", "texture"];
    const SILENT: f32 = -60.0;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Mix {
        db: [f32; 5],
        cutoff: f64,
        reverb: f32,
        damping: f64,
    }
    fn mix(mood: &str) -> Mix {
        match mood {
            "the High" => Mix {
                db: [0.0; 5],
                cutoff: 12000.0,
                reverb: 0.10,
                damping: 0.35,
            },
            "the Comedown" => Mix {
                db: [SILENT, -8.0, -10.0, SILENT, -12.0],
                cutoff: 1000.0,
                reverb: 0.02,
                damping: 0.85,
            },
            _ => Mix {
                db: [-3.0, -2.0, -3.0, -16.0, -4.0],
                cutoff: 4200.0,
                reverb: 0.035,
                damping: 0.7,
            },
        }
    }
    fn fade() -> Tween {
        Tween {
            duration: Duration::from_millis(1500),
            ..default()
        }
    }

    #[derive(Default)]
    pub(super) struct Music {
        player: Option<Player>,
        attempted: bool,
        applied: Option<(bool, Mix)>,
    }
    struct Player {
        track: TrackHandle,
        filter: FilterHandle,
        reverb: ReverbHandle,
        pending: Vec<StaticSoundData>,
        stems: Vec<StaticSoundHandle>,
    }
    impl Player {
        fn new(
            parent: &mut TrackHandle,
            pending: Vec<StaticSoundData>,
        ) -> Result<Self, Box<dyn Error>> {
            if pending.len() != STEMS.len() || pending[0].num_frames() == 0 {
                return Err("music needs five nonempty stems".into());
            }
            if pending
                .iter()
                .any(|s| s.num_frames() != pending[0].num_frames() || s.sample_rate != 44100)
            {
                return Err("music stems must have equal lengths at 44.1 kHz".into());
            }
            let mut builder = TrackBuilder::new()
                .volume(Decibels::SILENCE)
                .sound_capacity(5);
            let reverb =
                builder.add_effect(ReverbBuilder::new().feedback(0.65).mix(0.035).damping(0.7));
            let filter = builder.add_effect(FilterBuilder::new().cutoff(4200.0).resonance(0.0));
            let mut track = parent.add_sub_track(builder)?;
            // Wait for the audio thread to acknowledge this pause before adding
            // any sound. A muted but running track would lose sample alignment.
            track.pause(Tween {
                duration: Duration::ZERO,
                ..default()
            });
            Ok(Self {
                track,
                filter,
                reverb,
                pending,
                stems: Vec::new(),
            })
        }

        fn start_when_paused(&mut self) -> Result<bool, Box<dyn Error>> {
            if !self.pending.is_empty() {
                if self.track.state() != TrackPlaybackState::Paused {
                    return Ok(false);
                }
                for data in self.pending.drain(..) {
                    self.stems.push(
                        self.track
                            .play(data.loop_region(..).volume(Decibels::SILENCE))?,
                    );
                }
                // Kira drains the sound queue before processing this resume.
                // All five transports advance in the same first audio buffer.
                self.track.resume(Tween {
                    duration: Duration::ZERO,
                    ..default()
                });
            }
            Ok(true)
        }

        fn apply(&mut self, audible: bool, mix: Mix) {
            self.track
                .set_volume(if audible { 0.0 } else { SILENT }, fade());
            for (stem, db) in self.stems.iter_mut().zip(mix.db) {
                stem.set_volume(db, fade());
            }
            self.filter.set_cutoff(mix.cutoff, fade());
            self.reverb.set_mix(mix.reverb, fade());
            self.reverb.set_damping(mix.damping, fade());
        }
    }

    fn load() -> Result<Vec<StaticSoundData>, Box<dyn Error>> {
        let root =
            bevy::asset::io::file::FileAssetReader::get_base_path().join("assets/music/ballroom");
        STEMS
            .iter()
            .map(|name| {
                let bytes = std::fs::read(root.join(format!("{name}.wav")))?;
                Ok(StaticSoundData::from_cursor(std::io::Cursor::new(bytes))?)
            })
            .collect()
    }

    pub(super) fn update(
        audio: Option<NonSendMut<Audio>>,
        active: Option<Res<ActiveRoom>>,
        mood: Option<Res<Mood>>,
        mut music: NonSendMut<Music>,
    ) {
        let audible = active
            .as_ref()
            .and_then(|a| a.room.as_ref())
            .and_then(|r| r.music.as_deref())
            == Some("ballroom");
        let wanted = (audible, mix(mood.as_ref().map_or("neutral", |m| m.name)));
        if audible && !music.attempted {
            let Some(mut audio) = audio else { return };
            music.attempted = true;
            let Some(parent) = audio.music_track() else {
                return;
            };
            match load().and_then(|data| Player::new(parent, data)) {
                Ok(player) => music.player = Some(player),
                Err(error) => {
                    warn!("Ballroom music disabled: {error}; render it with `cargo xtask music`")
                }
            }
        }
        let changed = music.applied != Some(wanted);
        let Some(player) = &mut music.player else {
            return;
        };
        match player.start_when_paused() {
            Ok(false) => return,
            Err(error) => {
                warn!("Ballroom music disabled: {error}; render it with `cargo xtask music`");
                // Dropping the child track also removes any partially queued loop.
                music.player = None;
                return;
            }
            Ok(true) => {}
        }
        if changed {
            player.apply(wanted.0, wanted.1);
            music.applied = Some(wanted);
        }
        // Keep silent transports running outside the ballroom, so returning and
        // hot reloading a room never restarts or duplicates a stem.
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn paused_staging_keeps_stems_aligned_through_loops_and_moods() {
            use kira::{
                AudioManager, AudioManagerSettings, Frame,
                backend::mock::{MockBackend, MockBackendSettings},
            };
            let mut manager = AudioManager::<MockBackend>::new(AudioManagerSettings {
                backend_settings: MockBackendSettings { sample_rate: 44100 },
                ..default()
            })
            .unwrap();
            let mut parent = manager.add_sub_track(TrackBuilder::new()).unwrap();
            let data = StaticSoundData {
                sample_rate: 44100,
                frames: vec![Frame::from_mono(0.01); 4410].into(),
                settings: default(),
                slice: None,
            };
            let mut player = Player::new(&mut parent, vec![data; 5]).unwrap();
            assert!(!player.start_when_paused().unwrap());
            manager.backend_mut().on_start_processing();
            manager.backend_mut().process();
            assert!(player.start_when_paused().unwrap());
            assert_eq!(player.stems.len(), 5);
            for mood in ["neutral", "the High", "the Comedown"] {
                for audible in [true, false, true] {
                    player.apply(audible, mix(mood));
                    for _ in 0..256 {
                        manager.backend_mut().on_start_processing();
                        manager.backend_mut().process();
                    }
                    let position = player.stems[0].position();
                    assert!(position > 0.0 && position < 0.1);
                    assert!(player.stems.iter().all(|s| s.position() == position));
                    assert_eq!(player.track.num_sounds(), 5);
                }
            }
        }

        #[test]
        fn shipped_stems_decode_on_the_same_grid() {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/music/ballroom");
            // The stems are rendered, not committed: without `cargo xtask music` there is nothing to check.
            if !root.exists() {
                eprintln!("skipped: no rendered music; run `cargo xtask music`");
                return;
            }
            for stem in STEMS {
                let data = StaticSoundData::from_file(root.join(format!("{stem}.wav"))).unwrap();
                assert_eq!(data.sample_rate, 44100);
                assert_eq!(data.num_frames(), 44100 * 64);
            }
        }

        #[test]
        fn missing_audio_is_silent() {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, MusicPlugin));
            app.update();
            assert!(app.world().non_send::<Music>().player.is_none());
        }

        #[test]
        fn moods_thin_and_open_the_arrangement() {
            let neutral = mix("neutral");
            let high = mix("the High");
            let down = mix("the Comedown");
            assert!(neutral.db[3] < neutral.db[2]);
            assert_eq!(high.db, [0.0; 5]);
            assert_eq!(down.db[0], SILENT);
            assert_eq!(down.db[3], SILENT);
            assert!(down.cutoff < neutral.cutoff && neutral.cutoff < high.cutoff);
            assert!(high.reverb > neutral.reverb);
            assert!(down.db[2] < neutral.db[2] && down.db[4] < neutral.db[4]);
        }
    }
}
