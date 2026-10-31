//! Tuning values, loaded from `assets/tuning.ron` and reloaded while the game runs.
//!
//! Systems read the [`Tuning`] resource. It holds the defaults below until the file has
//! loaded, and is replaced whenever the file changes on disk (with the `dev` feature).
//! The movement numbers live in the `world` crate, so headless tests use the same defaults.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
pub use world::MoveTuning;

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

#[derive(Asset, Resource, TypePath, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Tuning {
    pub movement: MoveTuning,
    pub camera: CameraTuning,
    pub juice: Juice,
}

/// The camera. Rates are per second; distances are pixels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CameraTuning {
    /// How quickly the camera closes the horizontal distance to its target.
    pub follow_rate_x: f32,
    /// How quickly it closes the vertical distance.
    pub follow_rate_y: f32,
    /// How far ahead of the player the camera looks, in the direction they face.
    pub lead: f32,
    /// How fast the look-ahead swings round when the player turns, in pixels per second.
    pub lead_speed: f32,
    /// The camera ignores a jump's height until the player falls this far below
    /// where they last stood, or rises this far above it.
    pub fall_release: f32,
    pub rise_release: f32,
    /// How far above the player the camera centres, so more of the room above is visible.
    pub look_up: f32,
}

impl Default for CameraTuning {
    fn default() -> Self {
        Self {
            follow_rate_x: 6.0,
            follow_rate_y: 4.0,
            lead: 40.0,
            lead_speed: 90.0,
            fall_release: 40.0,
            rise_release: 100.0,
            look_up: 24.0,
        }
    }
}

/// Squash, stretch, afterimages and rumble.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Juice {
    /// How quickly a squashed or stretched sprite returns to its shape, per second.
    pub squash_recovery: f32,
    /// Stretch on a jump, as width and height scale.
    pub jump_stretch: (f32, f32),
    /// Squash on a hard landing.
    pub land_squash: (f32, f32),
    /// Stretch on a Lunge.
    pub lunge_stretch: (f32, f32),
    /// Seconds an afterimage of the Lunge lingers.
    pub afterimage_life: f32,
    /// Multiplies every rumble; 0 turns rumble off.
    pub rumble: f32,
}

impl Default for Juice {
    fn default() -> Self {
        Self {
            squash_recovery: 14.0,
            jump_stretch: (0.75, 1.3),
            land_squash: (1.4, 0.65),
            lunge_stretch: (1.45, 0.75),
            afterimage_life: 0.2,
            rumble: 1.0,
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
            && *tuning != *loaded
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The file and the code agree, so the game feels the same before and after the file loads.
    #[test]
    fn the_tuning_file_matches_the_defaults() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/tuning.ron"
        ))
        .unwrap();
        let file: Tuning = ron::from_str(&text).unwrap();
        assert_eq!(file, Tuning::default());
    }
}
