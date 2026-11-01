//! Prompts that say what can be done here, and with which button.
//!
//! A [`Prompt`] is a short line of text in the world, such as `[W] Talk`,
//! over something the player can use. Whoever owns a prompt shows and hides it;
//! this module writes its text for the device the player last used, keyboard or gamepad.
//! [`Control::key`] names a button the same way for menus, banners and the dialogue box.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::render::OVERLAY_LAYER;

pub struct PromptPlugin;

impl Plugin for PromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Device>()
            .add_systems(Update, (track_device, write).chain());
    }
}

/// The device the player last pressed something on.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Device {
    #[default]
    Gamepad,
    Keyboard,
}

/// A button the game asks the player to press, whichever device they hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Up,
    Jump,
    Lunge,
    Lure,
    Plunge,
    SwitchWeapon,
}

impl Control {
    /// The button's name on the device, as printed on it.
    pub fn label(self, device: Device) -> &'static str {
        use Control::*;
        match device {
            Device::Gamepad => match self {
                Up => "UP",
                Jump => "A",
                Lunge => "B",
                Lure => "RT",
                Plunge => "RB",
                SwitchWeapon => "Y",
            },
            Device::Keyboard => match self {
                Up => "W",
                Jump => "SPACE",
                Lunge => "SHIFT",
                Lure => "I",
                Plunge => "M",
                SwitchWeapon => "L",
            },
        }
    }

    /// The button in brackets, for a line of text: `[A]`, `[SPACE]`.
    pub fn key(self, device: Device) -> String {
        format!("[{}]", self.label(device))
    }
}

/// A line over something the player can use: which button, and what it does.
#[derive(Component)]
pub struct Prompt {
    pub button: Control,
    pub verb: &'static str,
}

impl Prompt {
    /// A hidden prompt centred at `at`, relative to its parent.
    pub fn bundle(button: Control, verb: &'static str, at: Vec2) -> impl Bundle {
        (
            Prompt { button, verb },
            Text2d::new(""),
            TextFont {
                font_size: bevy::text::FontSize::Px(8.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.88, 0.7)),
            RenderLayers::layer(OVERLAY_LAYER),
            Transform::from_translation(at.extend(5.0)),
            Visibility::Hidden,
        )
    }
}

/// Follows the last device pressed. Without a gamepad at startup, the keyboard is assumed.
fn track_device(
    mut device: ResMut<Device>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut started: Local<bool>,
) {
    if !*started {
        *started = true;
        if gamepads.is_empty() {
            *device = Device::Keyboard;
        }
    }
    let used = if keys.get_just_pressed().next().is_some() {
        Some(Device::Keyboard)
    } else if gamepads
        .iter()
        .any(|g| g.get_just_pressed().next().is_some())
    {
        Some(Device::Gamepad)
    } else {
        None
    };
    if let Some(used) = used
        && *device != used
    {
        *device = used;
    }
}

fn write(device: Res<Device>, mut prompts: Query<(Ref<Prompt>, &mut Text2d)>) {
    for (prompt, mut text) in &mut prompts {
        if !device.is_changed() && !prompt.is_added() {
            continue;
        }
        let line = format!("{} {}", prompt.button.key(*device), prompt.verb);
        if text.0 != line {
            text.0 = line;
        }
    }
}
