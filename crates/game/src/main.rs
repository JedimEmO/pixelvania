//! The game binary: app setup and plugin wiring.

mod ambience;
pub mod audio;
mod backdrop;
mod boss;
mod camera;
mod combat;
mod devtools;
mod effects;
mod encounter;
mod enemies;
mod fade;
mod fire;
mod hanging;
mod hud;
mod indulge;
mod input;
mod levels;
mod lounge;
mod lure;
mod map;
mod music;
mod player;
mod plunge;
mod progress;
mod projectiles;
mod render;
mod rooms;
mod story;
mod travel;
mod tuning;

use bevy::prelude::*;
use bevy::window::WindowResolution;

/// Gameplay ticks per second. Fixed, so jumps are the same height on every machine.
const TICK_HZ: f64 = 120.0;

fn main() -> AppExit {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "pixelvania".into(),
                        resolution: WindowResolution::new(1280, 720),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
        .add_plugins((
            render::PixelRenderPlugin,
            audio::AudioPlugin,
            music::MusicPlugin,
            backdrop::BackdropPlugin,
            tuning::TuningPlugin,
            input::InputPlugin,
            rooms::RoomsPlugin,
            player::PlayerPlugin,
            combat::CombatPlugin,
            enemies::EnemiesPlugin,
            hud::HudPlugin,
            fire::FirePlugin,
            camera::CameraPlugin,
            effects::EffectsPlugin,
            devtools::DevToolsPlugin,
        ))
        .add_plugins((
            boss::BossPlugin,
            lure::LurePlugin,
            hanging::HangingPlugin,
            fade::FadePlugin,
            travel::TravelPlugin,
            progress::ProgressPlugin,
            lounge::LoungePlugin,
            indulge::IndulgePlugin,
            plunge::PlungePlugin,
            projectiles::ProjectilesPlugin,
            map::MapPlugin,
            levels::LevelsPlugin,
            story::StoryPlugin,
            encounter::EncounterPlugin,
            ambience::AmbiencePlugin,
        ))
        .run()
}
