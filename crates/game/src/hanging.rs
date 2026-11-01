//! Chandeliers and lanterns: lights on chains that the Lure takes hold of.
//!
//! Each is painted by `assetgen::hanging` into a surface image, lit by the room like the tiles,
//! and a glow image for its flames. It sways on its chain like a damped pendulum.
//! A body on its rope leans it towards the body, and the moment the Lure takes hold
//! the flames flare and its light swells.
//!
//! The Lure's anchor stays where the room file puts it; the sway is small enough
//! that the rope, drawn to the swayed fixture, never shows the difference.

use assetgen::hanging::{self, Kind};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::{EntityDef, ParseError, Room, TILE};

use crate::player::Player;
use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::RoomPart;

pub struct HangingPlugin;

impl Plugin for HangingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sway, repaint).chain());
    }
}

/// A hanging light.
#[derive(Component)]
pub struct Hanging {
    kind: Kind,
    chain: u32,
    /// The grab point at rest, in room pixels with y down: the Lure's anchor.
    pub anchor: (f32, f32),
    /// Sway in radians, positive to the right, and its rate.
    angle: f32,
    spin: f32,
    /// From 1 just after the Lure takes hold, fading to 0.
    flare: f32,
    /// A body was on this light's rope last frame.
    held: bool,
    base_strength: f32,
    /// What the images were last painted with: angle in hundredths, step, flare on or off.
    painted: Option<(i32, u32, bool)>,
    color: Handle<Image>,
    glow: Handle<Image>,
    /// A seed so neighbouring lights do not sway in step.
    phase: f32,
}

impl Hanging {
    fn hang(&self) -> f32 {
        (self.chain as i32 + hanging::CROWN) as f32
    }

    /// The pivot at the top of the chain, in world space.
    fn pivot(&self) -> Vec2 {
        Vec2::new(self.anchor.0, -(self.anchor.1 - self.hang()))
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The grab point where it hangs now, swayed, in world space.
    pub fn grab_point(&self) -> Vec2 {
        let (s, c) = self.angle.sin_cos();
        self.pivot() + Vec2::new(s, -c) * self.hang()
    }
}

/// How much a fixture flares while the Lure is aimed at it: enough for tall flames.
const AIMED_FLARE: f32 = 0.5;

/// The light a fixture gives, its sway's stiffness, and how far a body on its rope leans it.
struct Feel {
    light: Light2d,
    stiffness: f32,
    lean: f32,
}

fn feel(kind: Kind) -> Feel {
    match kind {
        Kind::Chandelier => Feel {
            light: Light2d {
                color: Color::srgb_u8(0xff, 0xcf, 0x8a).to_linear(),
                radius: 200.0,
                strength: 1.7,
                glow: 1.0,
            },
            stiffness: 18.0,
            lean: 0.1,
        },
        Kind::Lantern => Feel {
            light: Light2d {
                color: Color::srgb_u8(0xff, 0xb0, 0x70).to_linear(),
                radius: 110.0,
                strength: 1.1,
                glow: 0.8,
            },
            stiffness: 30.0,
            lean: 0.22,
        },
    }
}

/// How long the chain is: from the crown up to the ceiling above, or `chain=` in the room file.
fn chain_length(room: &Room, def: &EntityDef) -> Result<u32, ParseError> {
    let (x, y) = def.pixel_pos();
    let crown = y - hanging::CROWN as f32;
    if def.props.contains_key("chain") {
        return Ok(def.num("chain", 0.0)?.max(0.0) as u32);
    }
    let tx = (x.floor() as i32).div_euclid(TILE);
    let mut ty = (crown.floor() as i32).div_euclid(TILE);
    while ty >= 0 && !room.is_solid(tx, ty) {
        ty -= 1;
    }
    let ceiling = ((ty + 1) * TILE) as f32;
    Ok((crown - ceiling).clamp(0.0, 400.0) as u32)
}

fn blank(chain: u32, images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new_fill(
        Extent3d {
            width: hanging::WIDTH,
            height: hanging::height(chain),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

/// Spawns the room's chandeliers and lanterns.
pub fn spawn(commands: &mut Commands, images: &mut Assets<Image>, room: &Room) {
    let kinds = [("chandelier", Kind::Chandelier), ("lantern", Kind::Lantern)];
    for (name, kind) in kinds {
        for (i, def) in room.entities_of(name).enumerate() {
            let read = || -> Result<(u32, Light2d), ParseError> {
                let base = feel(kind).light;
                let c = Srgba::from(base.color);
                let [r, g, b] = def.color("color", [c.red, c.green, c.blue])?;
                Ok((
                    chain_length(room, def)?,
                    Light2d {
                        color: Color::srgb(r, g, b).to_linear(),
                        radius: def.num("radius", base.radius)?,
                        strength: def.num("strength", base.strength)?,
                        glow: def.num("glow", base.glow)?,
                    },
                ))
            };
            let (chain, light) = match read() {
                Ok(v) => v,
                Err(e) => {
                    error!("room `{}`: {e}", room.name);
                    continue;
                }
            };
            let color = blank(chain, images);
            let glow = blank(chain, images);
            let h = Hanging {
                kind,
                chain,
                anchor: def.pixel_pos(),
                angle: 0.0,
                spin: 0.0,
                flare: 0.0,
                held: false,
                base_strength: light.strength,
                painted: None,
                color: color.clone(),
                glow: glow.clone(),
                phase: i as f32 * 2.3 + def.x * 0.37,
            };
            let pivot = h.pivot();
            let at = Vec2::new(-(hanging::WIDTH as f32) / 2.0, 0.0);
            let grab = h.grab_point();
            commands.spawn((
                RoomPart,
                Name::new(name),
                Transform::from_translation(pivot.extend(0.4)),
                Visibility::default(),
                children![
                    (
                        Sprite::from_image(color),
                        Anchor::TOP_LEFT,
                        Transform::from_translation(at.extend(0.0)),
                    ),
                    (
                        Sprite::from_image(glow),
                        Anchor::TOP_LEFT,
                        Transform::from_translation(at.extend(0.0)),
                        RenderLayers::layer(GLOW_LAYER),
                    ),
                ],
            ));
            commands.spawn((
                RoomPart,
                Name::new(format!("{name} light")),
                light,
                Transform::from_translation(grab.extend(0.0)),
                HangingLight(def.pixel_pos()),
            ));
            commands.spawn((RoomPart, h));
        }
    }
}

/// The light of the hanging fixture whose anchor this is, following its sway.
#[derive(Component)]
struct HangingLight((f32, f32));

/// Swings each fixture: a damped pendulum, idly breathing, leaned by a body on its rope.
fn sway(
    time: Res<Time>,
    players: Query<&Player>,
    aimed: Res<crate::lure::LureTarget>,
    mut fixtures: Query<&mut Hanging>,
    mut lights: Query<(&HangingLight, &mut Light2d, &mut Transform)>,
) {
    let dt = time.delta_secs().min(1.0 / 20.0);
    let t = time.elapsed_secs();
    for mut h in &mut fixtures {
        let f = feel(h.kind);
        let rope = players
            .iter()
            .find_map(|p| p.ctrl.rope.filter(|r| r.anchor == h.anchor).map(|r| (p, r)));
        let mut target = 0.012 * (t * 0.7 + h.phase).sin();
        if let Some((p, rope)) = rope {
            let hands = p.ctrl.hands();
            target += ((hands.0 - rope.anchor.0) / rope.length).clamp(-1.0, 1.0) * f.lean;
            if !h.held {
                // The catch: the flames flare and the fixture takes some of the body's swing.
                h.flare = 1.0;
                h.spin += p.ctrl.vx * 0.0012;
            }
        }
        h.held = rope.is_some();
        // The light the Lure is aimed at answers: its flames rise and its light swells.
        if aimed.anchor == Some(h.anchor) {
            h.flare = h.flare.max(AIMED_FLARE);
        }
        let accel = f.stiffness * (target - h.angle) - 2.5 * h.spin;
        h.spin += accel * dt;
        h.angle = (h.angle + h.spin * dt).clamp(-0.35, 0.35);
        h.flare = (h.flare - dt * 2.5).max(0.0);
        for (l, mut light, mut transform) in &mut lights {
            if l.0 != h.anchor {
                continue;
            }
            let step = (t * 12.0).floor() as u32;
            let flicker =
                0.95 + 0.1 * ((step.wrapping_mul(2_654_435_761) >> 16) % 100) as f32 / 100.0;
            light.strength = (h.base_strength * flicker * (1.0 + 0.45 * h.flare)).min(2.0);
            transform.translation = h.grab_point().extend(0.0);
        }
    }
}

/// Repaints a fixture when its sway, its flames' step or its flare has changed.
fn repaint(time: Res<Time>, mut fixtures: Query<&mut Hanging>, mut images: ResMut<Assets<Image>>) {
    let step = (time.elapsed_secs() * 12.0).floor() as u32;
    for mut h in &mut fixtures {
        // Sway shows on whole hundredths of a radian, on the animation's steps.
        let key = ((h.angle * 100.0).round() as i32, step, h.flare > 0.3);
        if h.painted == Some(key) {
            continue;
        }
        h.painted = Some(key);
        let p = hanging::paint(h.kind, h.chain, key.0 as f32 / 100.0, step, h.flare);
        if let Some(mut img) = images.get_mut(&h.color) {
            img.data = Some(p.color.into_raw());
        }
        if let Some(mut img) = images.get_mut(&h.glow) {
            img.data = Some(p.glow.into_raw());
        }
    }
}
