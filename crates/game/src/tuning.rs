//! Tuning values, loaded from `assets/tuning.ron` and reloaded while the game runs.
//!
//! Systems read the [`Tuning`] resource. It holds the defaults below until the file has
//! loaded, and is replaced whenever the file changes on disk (with the `dev` feature).

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::Deserialize;

pub struct TuningPlugin;

impl Plugin for TuningPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Tuning>()
            .register_asset_loader(TuningLoader)
            .init_resource::<Tuning>()
            .add_systems(Startup, load)
            .add_systems(PreUpdate, apply_changes);
    }
}

#[derive(Asset, Resource, TypePath, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Tuning {
    pub movement: Movement,
    pub camera: CameraTuning,
}

/// Player movement. Speeds in pixels per second, accelerations in pixels per second squared.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Movement {
    pub run_speed: f32,
    pub run_accel: f32,
    pub run_decel: f32,
    /// Acceleration while reversing direction on the ground.
    pub turn_accel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    pub gravity: f32,
    pub max_fall: f32,
    pub jump_speed: f32,
    /// Upward speed is multiplied by this when jump is released early.
    pub jump_cut: f32,
    /// Stick deflection needed to start walking.
    pub walk_threshold: f32,
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            run_speed: 150.0,
            run_accel: 1900.0,
            run_decel: 2400.0,
            turn_accel: 3200.0,
            air_accel: 1400.0,
            air_decel: 900.0,
            gravity: 950.0,
            max_fall: 340.0,
            jump_speed: 320.0,
            jump_cut: 0.4,
            walk_threshold: 0.3,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct CameraTuning {
    /// How quickly the camera closes the distance to its target, per second.
    pub follow_rate: f32,
    /// How far ahead of the player the camera looks, in pixels.
    pub lead: f32,
}

impl Default for CameraTuning {
    fn default() -> Self {
        Self {
            follow_rate: 6.0,
            lead: 40.0,
        }
    }
}

#[derive(Resource)]
struct TuningHandle(Handle<Tuning>);

fn load(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(TuningHandle(assets.load("tuning.ron")));
}

fn apply_changes(
    mut events: MessageReader<AssetEvent<Tuning>>,
    handle: Res<TuningHandle>,
    assets: Res<Assets<Tuning>>,
    mut tuning: ResMut<Tuning>,
) {
    for event in events.read() {
        let id = match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => *id,
            _ => continue,
        };
        if id == handle.0.id()
            && let Some(loaded) = assets.get(id)
        {
            *tuning = loaded.clone();
            info!("tuning loaded");
        }
    }
}

#[derive(TypePath, Default)]
struct TuningLoader;

impl AssetLoader for TuningLoader {
    type Asset = Tuning;
    type Settings = ();
    type Error = TuningError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut LoadContext<'_>,
    ) -> Result<Tuning, TuningError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

#[derive(Debug)]
enum TuningError {
    Io(std::io::Error),
    Ron(ron::error::SpannedError),
}

impl std::fmt::Display for TuningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TuningError::Io(e) => write!(f, "reading tuning: {e}"),
            TuningError::Ron(e) => write!(f, "parsing tuning: {e}"),
        }
    }
}

impl std::error::Error for TuningError {}

impl From<std::io::Error> for TuningError {
    fn from(e: std::io::Error) -> Self {
        TuningError::Io(e)
    }
}

impl From<ron::error::SpannedError> for TuningError {
    fn from(e: ron::error::SpannedError) -> Self {
        TuningError::Ron(e)
    }
}
