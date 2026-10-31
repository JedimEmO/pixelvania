//! Tools for checking the game without a person at the controls.
//!
//! `PIXELVANIA_SCREENSHOT=path.png` saves a screenshot after the game has settled, then exits.
//! `PIXELVANIA_HOLD=right,jump` holds those inputs from the start, for screenshots of motion.
//! The agents use these to look at what they built.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::input::TickInput;

pub struct DevToolsPlugin;

/// Frames to wait before the screenshot: long enough for assets to load and the camera to settle.
const SCREENSHOT_FRAME: u32 = 150;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(path) = std::env::var("PIXELVANIA_SCREENSHOT") {
            let frame_at = std::env::var("PIXELVANIA_SCREENSHOT_FRAME")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(SCREENSHOT_FRAME);
            app.insert_resource(ScreenshotPlan { path, frame_at })
                .add_systems(Update, screenshot_and_exit);
        }
        if let Ok(hold) = std::env::var("PIXELVANIA_HOLD") {
            let held: Vec<String> = hold.split(',').map(|s| s.trim().to_owned()).collect();
            app.insert_resource(HeldInputs(held))
                .add_systems(PreUpdate, hold_inputs.after(crate::input::collect));
        }
    }
}

#[derive(Resource)]
struct ScreenshotPlan {
    path: String,
    frame_at: u32,
}

#[derive(Resource)]
struct HeldInputs(Vec<String>);

fn screenshot_and_exit(
    mut commands: Commands,
    plan: Res<ScreenshotPlan>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == plan.frame_at {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(plan.path.clone()));
    }
    // Saving happens a few frames later, on another thread.
    if *frame == plan.frame_at + 30 {
        exit.write(AppExit::Success);
    }
}

fn hold_inputs(held: Res<HeldInputs>, mut input: ResMut<TickInput>, mut jumped: Local<bool>) {
    for h in &held.0 {
        match h.as_str() {
            "left" => input.move_axis.x = -1.0,
            "right" => input.move_axis.x = 1.0,
            "jump" => {
                input.jump_held = true;
                if !*jumped {
                    input.jump_pressed = true;
                    *jumped = true;
                }
            }
            _ => {}
        }
    }
}
