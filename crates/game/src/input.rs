//! Gamepad-first input, buffered for the fixed gameplay tick.
//!
//! Input is read once per rendered frame, but gameplay runs at a fixed 120 ticks per second,
//! so a frame may run zero, one or several ticks. A press is recorded in [`TickInput`] when it
//! happens and stays there until a tick consumes it, so a tap is never lost between ticks.
//!
//! At the start of each tick, [`TickInput`] becomes the tick's [`TickControls`].
//! Recording and replay hook in between, in [`TickSet::Input`].

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use world::Controls;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<Gameplay>()
            .init_resource::<TickInput>()
            .init_resource::<TickControls>()
            .configure_sets(FixedUpdate, (TickSet::Input, TickSet::Simulate).chain())
            .add_systems(Startup, spawn_controls)
            // After `Apply`, which copies this frame's values onto the typed actions;
            // after only `Update`, held buttons and the stick would read a frame late.
            .add_systems(PreUpdate, collect.after(EnhancedInputSystems::Apply))
            .add_systems(FixedUpdate, take_controls.in_set(TickSet::Input));
    }
}

/// The order of work inside a fixed tick.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TickSet {
    /// [`TickControls`] is filled in, recorded or replaced by a replay.
    Input,
    /// Gameplay reads [`TickControls`].
    Simulate,
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

#[derive(InputAction)]
#[action_output(bool)]
pub struct Lunge;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Attack;

/// The Toast: a parry timed against a blow.
#[derive(InputAction)]
#[action_output(bool)]
pub struct Toast;

/// The Lure: a grapple that only takes hold of light. Held to stay on the rope.
#[derive(InputAction)]
#[action_output(bool)]
pub struct Lure;

/// Indulge: tap to heal with Nectar, hold to start a High.
#[derive(InputAction)]
#[action_output(bool)]
pub struct Indulge;

/// The Plunge: a dive straight down that breaks cracked floors.
#[derive(InputAction)]
#[action_output(bool)]
pub struct Plunge;

/// Switches between the weapons.
#[derive(InputAction)]
#[action_output(bool)]
pub struct SwitchWeapon;

/// Goes to the next test room: the feel lab or the combat lab.
#[derive(InputAction)]
#[action_output(bool)]
pub struct NextRoom;

/// Puts the player back at the room's spawn point. For the feel lab.
#[derive(InputAction)]
#[action_output(bool)]
pub struct Reset;

/// Input gathered since the last tick.
#[derive(Resource, Default, Debug)]
pub struct TickInput {
    /// Stick or d-pad, x right, y up, each -1..1.
    pub move_axis: Vec2,
    pub jump_held: bool,
    /// Jump was pressed since the last tick consumed it.
    pub jump_pressed: bool,
    pub lunge_pressed: bool,
    pub attack_held: bool,
    pub attack_pressed: bool,
    pub toast_pressed: bool,
    pub lure_held: bool,
    pub lure_pressed: bool,
    pub plunge_pressed: bool,
    /// Held this frame. Read in `Update`, not by the tick.
    pub indulge_held: bool,
    /// These were pressed this frame. Read in `Update`, not by the tick.
    pub reset_pressed: bool,
    pub switch_weapon_pressed: bool,
    pub next_room_pressed: bool,
}

/// The input of the tick being simulated.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct TickControls(pub Controls);

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
            --
            Action::<Lunge>::new()
            Bindings [
                Binding(GamepadButton::East)
                --
                Binding(KeyCode::ShiftLeft)
            ]
            --
            Action::<Attack>::new()
            Bindings [
                Binding(GamepadButton::West)
                --
                Binding(KeyCode::KeyJ)
            ]
            --
            Action::<Toast>::new()
            Bindings [
                Binding(GamepadButton::LeftTrigger)
                --
                Binding(KeyCode::KeyK)
            ]
            --
            Action::<Lure>::new()
            Bindings [
                Binding(GamepadButton::RightTrigger2)
                --
                Binding(KeyCode::KeyI)
            ]
            --
            Action::<Indulge>::new()
            Bindings [
                Binding(GamepadButton::LeftTrigger2)
                --
                Binding(KeyCode::KeyN)
            ]
            --
            Action::<Plunge>::new()
            Bindings [
                Binding(GamepadButton::RightTrigger)
                --
                Binding(KeyCode::KeyM)
            ]
            --
            Action::<SwitchWeapon>::new()
            Bindings [
                Binding(GamepadButton::North)
                --
                Binding(KeyCode::KeyL)
            ]
            --
            Action::<NextRoom>::new()
            Bindings [
                Binding(GamepadButton::Start)
                --
                Binding(KeyCode::Tab)
            ]
            --
            Action::<Reset>::new()
            Bindings [
                Binding(GamepadButton::Select)
                --
                Binding(KeyCode::KeyR)
            ]
        ]
    });
}

pub fn collect(
    mut input: ResMut<TickInput>,
    movement: Single<&Action<Move>>,
    jump: Single<(&Action<Jump>, &ActionEvents)>,
    lunge: Single<&ActionEvents, With<Action<Lunge>>>,
    reset: Single<&ActionEvents, With<Action<Reset>>>,
    attack: Single<(&Action<Attack>, &ActionEvents)>,
    toast: Single<&ActionEvents, With<Action<Toast>>>,
    lure: Single<(&Action<Lure>, &ActionEvents)>,
    indulge: Single<&Action<Indulge>>,
    plunge: Single<&ActionEvents, With<Action<Plunge>>>,
    switch: Single<&ActionEvents, With<Action<SwitchWeapon>>>,
    next_room: Single<&ActionEvents, With<Action<NextRoom>>>,
) {
    input.move_axis = ***movement;
    let (held, events) = *jump;
    input.jump_held = **held;
    input.jump_pressed |= events.contains(ActionEvents::START);
    input.lunge_pressed |= lunge.contains(ActionEvents::START);
    input.reset_pressed = reset.contains(ActionEvents::START);
    let (held, events) = *attack;
    input.attack_held = **held;
    input.attack_pressed |= events.contains(ActionEvents::START);
    input.toast_pressed |= toast.contains(ActionEvents::START);
    let (held, events) = *lure;
    input.lure_held = **held;
    input.lure_pressed |= events.contains(ActionEvents::START);
    input.indulge_held = ***indulge;
    input.plunge_pressed |= plunge.contains(ActionEvents::START);
    input.switch_weapon_pressed = switch.contains(ActionEvents::START);
    input.next_room_pressed = next_room.contains(ActionEvents::START);
}

/// Hands the buffered input to this tick, consuming the presses.
pub fn take_controls(mut input: ResMut<TickInput>, mut controls: ResMut<TickControls>) {
    controls.0 = Controls {
        x: input.move_axis.x,
        y: input.move_axis.y,
        jump_held: input.jump_held,
        jump_pressed: std::mem::take(&mut input.jump_pressed),
        lunge_pressed: std::mem::take(&mut input.lunge_pressed),
        attack_pressed: std::mem::take(&mut input.attack_pressed),
        attack_held: input.attack_held,
        parry_pressed: std::mem::take(&mut input.toast_pressed),
        lure_pressed: std::mem::take(&mut input.lure_pressed),
        lure_held: input.lure_held,
        plunge_pressed: std::mem::take(&mut input.plunge_pressed),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A press must read as held on the same frame it starts.
    /// Reading the typed action before the library applies its value lags held buttons
    /// and the stick by a frame, and a jump started on that frame is cut short at once.
    #[test]
    fn a_press_reads_as_held_on_the_frame_it_starts() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::input::InputPlugin,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            InputPlugin,
        ));
        app.finish();
        app.cleanup();
        app.update();

        let set_space = |app: &mut App, down: bool| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            if down {
                keys.press(KeyCode::Space);
            } else {
                keys.release(KeyCode::Space);
            }
            app.update();
            let i = app.world().resource::<TickInput>();
            (i.jump_held, i.jump_pressed)
        };
        assert_eq!(set_space(&mut app, true), (true, true));
        app.world_mut().resource_mut::<TickInput>().jump_pressed = false;
        assert_eq!(set_space(&mut app, true), (true, false));
        assert_eq!(set_space(&mut app, false), (false, false));
    }
}
