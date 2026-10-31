//! Gamepad-first input, buffered for the fixed gameplay tick.
//!
//! Input is read once per rendered frame, but gameplay runs at a fixed 120 ticks per second,
//! so a frame may run zero, one or several ticks. A press is recorded in [`TickInput`] when it
//! happens and stays there until a tick consumes it, so a tap is never lost between ticks.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<Gameplay>()
            .init_resource::<TickInput>()
            .add_systems(Startup, spawn_controls)
            .add_systems(PreUpdate, collect.after(EnhancedInputSystems::Update));
    }
}

/// The input context for moving and fighting.
#[derive(Component, Default, Clone)]
pub struct Gameplay;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Move;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Jump;

/// Input as the gameplay tick sees it.
#[derive(Resource, Default, Debug)]
pub struct TickInput {
    /// Stick or d-pad, x right, y up, each -1..1.
    pub move_axis: Vec2,
    pub jump_held: bool,
    /// Jump was pressed since the last tick consumed it.
    pub jump_pressed: bool,
}

fn spawn_controls(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Gameplay
        Actions<Gameplay> [
            Action::<Move>::new()
            DeadZone
            Bindings [
                {preset::axial::left_stick()}
                --
                {preset::cardinal::dpad()}
                --
                {preset::cardinal::wasd_keys()}
                --
                {preset::cardinal::arrows()}
            ]
            --
            Action::<Jump>::new()
            Bindings [
                Binding(GamepadButton::South)
                --
                Binding(KeyCode::Space)
            ]
        ]
    });
}

pub fn collect(
    mut input: ResMut<TickInput>,
    movement: Single<&Action<Move>>,
    jump: Single<(&Action<Jump>, &ActionEvents)>,
) {
    input.move_axis = ***movement;
    let (held, events) = *jump;
    input.jump_held = **held;
    if events.contains(ActionEvents::START) {
        input.jump_pressed = true;
    }
}
