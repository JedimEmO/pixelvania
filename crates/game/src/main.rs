//! The game binary: app setup and plugin wiring.

pub mod audio;
mod backdrop;
mod camera;
mod combat;
mod devtools;
mod effects;
mod enemies;
mod fire;
mod hud;
mod input;
mod player;
mod render;
mod rooms;
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
        .run()
}
