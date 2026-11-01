//! Named room and encounter stems beneath the ducked music bus.
//!
//! A room names its music with `music = ballroom`, and `music = ballroom muffled` hears it
//! through a wall: quieter, darker and drier. Walking from a muffled room into an open one
//! fades the filter open, so the party comes into the room with the player.
use bevy::prelude::*;

pub struct MusicPlugin;
impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Encounter>()
            .add_systems(Startup, encounter_from_env)
            .init_resource::<Beat>()
            .add_systems(PreUpdate, advance_beat);
        stems::build(app);
        app.insert_non_send(native::Music::default())
            .add_systems(PostUpdate, native::update);
    }
}

/// Overrides room music while a boss encounter is active.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct Encounter {
    pub track: Option<String>,
    pub phase: u8,
}

fn encounter_from_env(mut encounter: ResMut<Encounter>) {
    if let Ok(value) = std::env::var("PIXELVANIA_ENCOUNTER") {
        if let Some((track, phase)) = value.rsplit_once(':')
            && !track.is_empty()
            && let Ok(phase @ 1..=3) = phase.parse::<u8>()
        {
            *encounter = Encounter {
                track: Some(track.into()),
                phase,
            };
        } else {
            warn!("Invalid PIXELVANIA_ENCOUNTER; expected track:1..3");
        }
    }
}

/// Where the music is, in seconds, so the room can move on its beat.
/// It follows the stems' own playback position while the ballroom music plays,
/// and runs on real time when there is no music, so lights still pulse in silence.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct Beat {
    pub seconds: f64,
}

impl Beat {
    /// The ballroom's tempo; `assetgen::music` renders it at this tempo.
    pub const BPM: f64 = 120.0;

    /// How far through the current beat, from 0 on the beat to 1 just before the next.
    pub fn phase(&self) -> f32 {
        (self.seconds * Self::BPM / 60.0).fract() as f32
    }
}

fn advance_beat(time: Res<Time<Real>>, mut beat: ResMut<Beat>) {
    beat.seconds += time.delta_secs_f64();
}

/// Where the stems come from: WAV files read at once on desktop,
/// OGG files fetched and decoded a stem a frame on the web.
#[cfg(not(target_arch = "wasm32"))]
mod stems {
    use super::*;
    use kira::sound::static_sound::StaticSoundData;
    use std::error::Error;

    pub(super) fn build(app: &mut App) {
        app.init_resource::<Stems>();
    }

    #[derive(Resource, Default)]
    pub(super) struct Stems;

    impl Stems {
        /// A track's stems, or `None` while they are still arriving.
        pub(super) fn take(
            &mut self,
            name: &str,
        ) -> Result<Option<Vec<StaticSoundData>>, Box<dyn Error>> {
            let root = bevy::asset::io::file::FileAssetReader::get_base_path()
                .join("assets/music")
                .join(name);
            native::catalog(name)
                .ok_or("unknown music track")?
                .0
                .iter()
                .map(|stem| {
                    let bytes = std::fs::read(root.join(format!("{stem}.wav")))?;
                    Ok(StaticSoundData::from_cursor(std::io::Cursor::new(bytes))?)
                })
                .collect::<Result<_, _>>()
                .map(Some)
        }
    }
}

/// On the web the stems are OGG Vorbis, encoded from the rendered WAVs when the site is built.
/// All of them are fetched at startup, and one is decoded each frame as it arrives,
/// so no single frame stalls for long.
#[cfg(target_arch = "wasm32")]
mod stems {
    use super::*;
    use bevy::asset::{AssetLoader, LoadContext, LoadState, io::Reader};
    use kira::sound::static_sound::StaticSoundData;
    use std::error::Error;

    /// Every track, in the order they are decoded: the rooms' music first.
    const TRACKS: [&str; 2] = ["ballroom", "dancer"];

    pub(super) fn build(app: &mut App) {
        app.init_asset::<StemBytes>()
            .register_asset_loader(StemLoader)
            .init_resource::<Stems>()
            .add_systems(Startup, request)
            .add_systems(Update, decode);
    }

    #[derive(Asset, TypePath)]
    struct StemBytes(Vec<u8>);

    #[derive(TypePath, Default)]
    struct StemLoader;

    impl AssetLoader for StemLoader {
        type Asset = StemBytes;
        type Settings = ();
        type Error = std::io::Error;

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &(),
            _context: &mut LoadContext<'_>,
        ) -> Result<StemBytes, std::io::Error> {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            Ok(StemBytes(bytes))
        }

        fn extensions(&self) -> &[&str] {
            &["ogg"]
        }
    }

    struct Track {
        name: &'static str,
        files: Vec<Handle<StemBytes>>,
        decoded: Vec<Option<StaticSoundData>>,
        failed: Option<String>,
    }

    #[derive(Resource, Default)]
    pub(super) struct Stems(Vec<Track>);

    impl Stems {
        /// A track's stems, or `None` while they are still arriving.
        pub(super) fn take(
            &mut self,
            name: &str,
        ) -> Result<Option<Vec<StaticSoundData>>, Box<dyn Error>> {
            let track = self
                .0
                .iter()
                .find(|t| t.name == name)
                .ok_or("unknown music track")?;
            if let Some(error) = &track.failed {
                return Err(error.clone().into());
            }
            Ok(track.decoded.iter().cloned().collect())
        }
    }

    fn request(server: Res<AssetServer>, mut stems: ResMut<Stems>) {
        for name in TRACKS {
            let Some((parts, _)) = native::catalog(name) else {
                continue;
            };
            stems.0.push(Track {
                name,
                files: parts
                    .iter()
                    .map(|part| server.load(format!("music/{name}/{part}.ogg")))
                    .collect(),
                decoded: vec![None; parts.len()],
                failed: None,
            });
        }
    }

    fn decode(
        server: Res<AssetServer>,
        mut files: ResMut<Assets<StemBytes>>,
        mut stems: ResMut<Stems>,
    ) {
        for track in &mut stems.0 {
            if track.failed.is_some() {
                continue;
            }
            for (file, decoded) in track.files.iter().zip(&mut track.decoded) {
                if decoded.is_some() {
                    continue;
                }
                if let LoadState::Failed(error) = server.load_state(file) {
                    track.failed = Some(error.to_string());
                    break;
                }
                let Some(bytes) = files.remove(file) else {
                    continue;
                };
                match StaticSoundData::from_cursor(std::io::Cursor::new(bytes.0)) {
                    Ok(data) => {
                        *decoded = Some(data);
                        if track.decoded.iter().all(Option::is_some) {
                            info!("music `{}` is ready", track.name);
                        }
                    }
                    Err(error) => track.failed = Some(error.to_string()),
                }
                return;
            }
        }
    }
}

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
    use std::{
        collections::{HashMap, HashSet},
        error::Error,
        time::Duration,
    };

    const STEMS: [&str; 5] = ["drums", "bass", "harmony", "lead", "texture"];
    const DANCER: [&str; 8] = [
        "drums", "bass", "pads", "arp", "stabs", "lead", "choir", "surge",
    ];
    pub(super) fn catalog(name: &str) -> Option<(&'static [&'static str], f64)> {
        match name {
            "ballroom" => Some((&STEMS, 120.0)),
            "dancer" => Some((&DANCER, 132.0)),
            _ => None,
        }
    }
    fn volumes(name: &str, phase: u8, mix: Mix) -> Vec<f32> {
        if name != "dancer" {
            return mix.db.to_vec();
        }
        [0, 1, 2, 2, 2, 3, 4, 0]
            .into_iter()
            .enumerate()
            .map(|(i, role)| {
                if i < [3, 5, 8][usize::from(phase.clamp(1, 3) - 1)] {
                    mix.db[role]
                } else {
                    SILENT
                }
            })
            .collect()
    }
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
    /// How far a full progression fade lowers each stem, in decibels:
    /// the warmth goes first, and the drums keep going to the end.
    const THIN: [f32; 5] = [0.0, -3.0, -8.0, -6.0, -12.0];

    /// The mix heard through a wall: quieter, with the highs gone and almost no room in it.
    fn muffled(m: Mix) -> Mix {
        Mix {
            db: m.db.map(|db| (db - 8.0).max(SILENT)),
            cutoff: m.cutoff.min(650.0),
            reverb: m.reverb * 0.3,
            damping: 0.9,
        }
    }

    /// A room's music: the track's name, and whether it is heard through a wall.
    fn room_music(spec: &str) -> (String, bool) {
        let mut words = spec.split_whitespace();
        let name = words.next().unwrap_or_default().to_owned();
        (name, words.any(|w| w == "muffled"))
    }

    /// The mood's mix, thinned by the progression fade as it shows in that mood.
    fn thinned(mood: &str, fade: f32) -> Mix {
        let mut m = mix(mood);
        // Whole twentieths, so a fade easing in does not retune the mix every frame.
        let fade = (fade * 20.0).round() / 20.0;
        for (db, thin) in m.db.iter_mut().zip(THIN) {
            *db = (*db + thin * fade).max(SILENT);
        }
        m
    }

    fn fade() -> Tween {
        Tween {
            duration: Duration::from_millis(1500),
            ..default()
        }
    }

    #[derive(Default)]
    pub(super) struct Music {
        players: HashMap<String, Player>,
        attempted: HashSet<String>,
        selected: Option<String>,
        encounter: bool,
    }
    struct Player {
        track: TrackHandle,
        filter: FilterHandle,
        reverb: ReverbHandle,
        pending: Vec<StaticSoundData>,
        stems: Vec<StaticSoundHandle>,
        applied: Option<(Vec<f32>, Mix)>,
        phase: u8,
    }
    impl Player {
        fn new(
            parent: &mut TrackHandle,
            pending: Vec<StaticSoundData>,
        ) -> Result<Self, Box<dyn Error>> {
            if pending.is_empty() || pending[0].num_frames() == 0 {
                return Err("music needs nonempty stems".into());
            }
            if pending
                .iter()
                .any(|s| s.num_frames() != pending[0].num_frames() || s.sample_rate != 44100)
            {
                return Err("music stems must have equal lengths at 44.1 kHz".into());
            }
            let mut builder = TrackBuilder::new()
                .volume(Decibels::SILENCE)
                .sound_capacity(pending.len());
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
                applied: None,
                phase: 1,
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

        fn set_mix(&mut self, db: Vec<f32>, mix: Mix) {
            if self.applied.as_ref() == Some(&(db.clone(), mix)) {
                return;
            }
            for (stem, db) in self.stems.iter_mut().zip(&db) {
                stem.set_volume(*db, fade());
            }
            self.filter.set_cutoff(mix.cutoff, fade());
            self.reverb.set_mix(mix.reverb, fade());
            self.reverb.set_damping(mix.damping, fade());
            self.applied = Some((db, mix));
        }
        #[cfg(test)]
        fn apply(&mut self, audible: bool, mix: Mix) {
            self.track
                .set_volume(if audible { 0.0 } else { SILENT }, fade());
            self.set_mix(mix.db.to_vec(), mix);
        }
    }

    fn transition(
        players: &mut HashMap<String, Player>,
        previous: Option<&str>,
        next: Option<&str>,
        quantized: bool,
    ) {
        let delay = if quantized {
            previous
                .and_then(|name| {
                    let position = players.get(name)?.stems.first()?.position();
                    let bar = 240.0 / catalog(name)?.1;
                    Some(bar - position.rem_euclid(bar))
                })
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let tween = Tween {
            start_time: Duration::from_secs_f64(delay).into(),
            duration: Duration::from_millis(if quantized { 1000 } else { 1500 }),
            ..default()
        };
        for (name, player) in players {
            player.track.set_volume(
                if Some(name.as_str()) == next {
                    0.0
                } else {
                    SILENT
                },
                tween,
            );
        }
    }

    pub(super) fn update(
        mut audio: Option<NonSendMut<Audio>>,
        active: Option<Res<ActiveRoom>>,
        mood: Option<Res<Mood>>,
        progression: Option<Res<crate::fade::ProgressionFade>>,
        encounter: Res<Encounter>,
        mut beat: ResMut<Beat>,
        mut music: NonSendMut<Music>,
        mut stems: ResMut<super::stems::Stems>,
    ) {
        let (room, through_wall) = active
            .as_ref()
            .and_then(|a| a.room.as_ref())
            .and_then(|r| r.music.as_deref())
            .map(room_music)
            .map_or((None, false), |(name, muffled)| (Some(name), muffled));
        let wanted = encounter.track.clone().or(room.clone());
        if let Some(name) = &wanted
            && !music.attempted.contains(name)
        {
            let Some(parent) = audio.as_mut().and_then(|a| a.music_track()) else {
                return;
            };
            // On the web the stems may still be arriving; ask again next frame.
            match stems.take(name) {
                Ok(None) => {}
                result => {
                    music.attempted.insert(name.clone());
                    match result.and_then(|data| Player::new(parent, data.unwrap_or_default())) {
                        Ok(player) => {
                            music.players.insert(name.clone(), player);
                        }
                        Err(error) => warn!(
                            "Music {name} unavailable: {error}; render with `cargo xtask music`"
                        ),
                    }
                }
            }
        }
        let mood = mood.map_or(Mood::ALL[0], |m| *m);
        let shown = progression.map_or(0.0, |f| f.shown(&mood));
        let mix = thinned(mood.name, shown);
        let mut ready = wanted.is_none();
        music
            .players
            .retain(|name, player| match player.start_when_paused() {
                Ok(true) => {
                    if encounter.track.as_ref() == Some(name) {
                        player.phase = encounter.phase;
                    }
                    let mix =
                        if through_wall && encounter.track.is_none() && room.as_ref() == Some(name)
                        {
                            muffled(mix)
                        } else {
                            mix
                        };
                    player.set_mix(volumes(name, player.phase, mix), mix);
                    if Some(name) == wanted.as_ref() {
                        ready = true;
                    }
                    true
                }
                Ok(false) => true,
                Err(error) => {
                    warn!("Music {name} disabled: {error}");
                    false
                }
            });
        if ready && music.selected != wanted {
            let previous = music.selected.clone();
            let quantized = music.encounter || encounter.track.is_some();
            transition(
                &mut music.players,
                previous.as_deref(),
                wanted.as_deref(),
                quantized,
            );
            music.selected = wanted;
            music.encounter = encounter.track.is_some();
        }
        if let Some(player) = music
            .selected
            .as_ref()
            .and_then(|name| music.players.get(name))
            && let Some(stem) = player.stems.first()
        {
            beat.seconds = stem.position();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_room_may_hear_its_music_through_a_wall() {
            assert_eq!(room_music("ballroom"), ("ballroom".into(), false));
            assert_eq!(room_music("ballroom muffled"), ("ballroom".into(), true));
            let open = mix("neutral");
            let through = muffled(open);
            assert!(through.cutoff < open.cutoff);
            assert!(
                through
                    .db
                    .iter()
                    .zip(open.db)
                    .all(|(m, o)| *m < o || *m == SILENT)
            );
        }

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
        fn named_switches_and_phases_preserve_alignment() {
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
            let mut players = HashMap::new();
            for (name, count) in [("ballroom", 5), ("dancer", 8)] {
                players.insert(
                    name.to_owned(),
                    Player::new(&mut parent, vec![data.clone(); count]).unwrap(),
                );
            }
            manager.backend_mut().on_start_processing();
            manager.backend_mut().process();
            for player in players.values_mut() {
                assert!(player.start_when_paused().unwrap());
            }
            for (previous, next) in [
                (None, Some("ballroom")),
                (Some("ballroom"), Some("dancer")),
                (Some("dancer"), Some("ballroom")),
            ] {
                transition(&mut players, previous, next, true);
                for phase in 1..=3 {
                    let high = mix("the High");
                    let expected: Vec<_> = (0..8)
                        .map(|i| {
                            if i < [3, 5, 8][phase - 1] {
                                0.0
                            } else {
                                SILENT
                            }
                        })
                        .collect();
                    let db = volumes("dancer", phase as u8, high);
                    assert_eq!(db, expected);
                    players.get_mut("dancer").unwrap().set_mix(db, high);
                    assert_eq!(players["dancer"].applied.as_ref().unwrap().0, expected);
                    for _ in 0..1536 {
                        manager.backend_mut().on_start_processing();
                        manager.backend_mut().process();
                    }
                    for player in players.values() {
                        let position = player.stems[0].position();
                        assert!(player.stems.iter().all(|s| s.position() == position));
                    }
                }
            }
            let down = volumes("dancer", 3, mix("the Comedown"));
            assert_eq!(down[0], SILENT);
            assert_eq!(down[5], SILENT);
            assert_eq!(down[7], SILENT);
            assert_eq!(
                volumes("dancer", 0, mix("the High")),
                volumes("dancer", 1, mix("the High"))
            );
            assert_eq!(
                volumes("dancer", 255, mix("the High")),
                volumes("dancer", 3, mix("the High"))
            );
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
        fn dancer_stems_decode_on_the_same_grid() {
            let root =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/music/dancer");
            if !root.exists() {
                return;
            }
            for stem in DANCER {
                let data = StaticSoundData::from_file(root.join(format!("{stem}.wav"))).unwrap();
                assert_eq!(data.sample_rate, 44100);
                assert_eq!(
                    data.num_frames(),
                    (44100.0_f64 * 240.0 / 132.0 * 40.0).round() as usize
                );
            }
        }

        #[test]
        fn missing_audio_is_silent() {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, MusicPlugin));
            app.update();
            assert!(app.world().non_send::<Music>().players.is_empty());
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
