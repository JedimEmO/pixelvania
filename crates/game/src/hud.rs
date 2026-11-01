//! The HUD in the top-left corner: health, poise, Cordials, Nectar and Favours,
//! and a line saying what the left trigger would do now.
//!
//! Everything is drawn into the overlay buffer, over the room, its lighting and its foreground,
//! and follows the view each frame.
//!
//! From the top: the health bar, the poise bar under it, a pip for each Cordial,
//! and the Nectar bar, marked where a High becomes affordable and brighter once it is.
//! Favours not yet spent show to the right, and pulse when one is gained.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::combat::Fighting;
use crate::indulge::{HIGH_COST, Indulgence, Phase};
use crate::player::Player;
use crate::progress::PlayerProgress;
use crate::prompt::{Control, Device};
use crate::render::{CameraFocus, HEIGHT, OVERLAY_LAYER, WIDTH};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(
            PostUpdate,
            (bars, pips, favours, hint, follow)
                .chain()
                .before(TransformSystems::Propagate),
        );
    }
}

/// Bar length in pixels when full, and the gap from the screen's corner.
const LENGTH: f32 = 60.0;
const MARGIN: f32 = 10.0;
/// The most Cordial pips drawn.
const PIPS: usize = 8;
const PIP: Vec2 = Vec2::new(3.0, 4.0);
const PIP_GAP: f32 = 2.0;

const TRACK: Color = Color::srgb(0.12, 0.1, 0.16);
const HEALTH: Color = Color::srgb(0.75, 0.22, 0.32);
const POISE: Color = Color::srgb(0.55, 0.5, 0.62);
const CORDIAL: Color = Color::srgb(0.95, 0.38, 0.45);
const NECTAR: Color = Color::srgb(0.75, 0.56, 0.22);
const NECTAR_READY: Color = Color::srgb(1.0, 0.82, 0.38);
const CREAM: Color = Color::srgb(1.0, 0.88, 0.7);

/// Where a HUD piece sits, in pixels right of and below the screen's top-left corner, and its depth.
#[derive(Component)]
struct At(Vec2, f32);

#[derive(Component)]
enum Bar {
    Health,
    Poise,
    Nectar,
}

#[derive(Component)]
struct Pip(usize);

#[derive(Component)]
struct FavourGlyph;

#[derive(Component)]
struct FavourCount;

#[derive(Component)]
struct Hint;

fn spawn(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let layer = RenderLayers::layer(OVERLAY_LAYER);
    let corner = |x: f32, y: f32, z: f32| At(Vec2::new(MARGIN + x, MARGIN + y), z);
    let bars = [
        (Bar::Health, HEALTH, 0.0, 3.0),
        (Bar::Poise, POISE, 4.0, 1.0),
        (Bar::Nectar, NECTAR, 13.0, 2.0),
    ];
    for (bar, color, y, h) in bars {
        // A dim track under each bar, so an empty bar is still visible.
        commands.spawn((
            Sprite::from_color(TRACK, Vec2::new(LENGTH, h)),
            Anchor::TOP_LEFT,
            Transform::default(),
            layer.clone(),
            corner(0.0, y, 5.0),
        ));
        commands.spawn((
            bar,
            Sprite::from_color(color, Vec2::new(LENGTH, h)),
            Anchor::TOP_LEFT,
            Transform::default(),
            layer.clone(),
            corner(0.0, y, 5.1),
        ));
    }
    // The mark on the Nectar bar where a High can be afforded.
    commands.spawn((
        Sprite::from_color(CREAM, Vec2::new(1.0, 4.0)),
        Anchor::TOP_LEFT,
        Transform::default(),
        layer.clone(),
        corner((LENGTH * HIGH_COST / 100.0).round(), 12.0, 5.2),
    ));
    for i in 0..PIPS {
        commands.spawn((
            Pip(i),
            Sprite::from_color(CORDIAL, PIP),
            Anchor::TOP_LEFT,
            Transform::default(),
            Visibility::Hidden,
            layer.clone(),
            corner(i as f32 * (PIP.x + PIP_GAP), 7.0, 5.1),
        ));
    }
    let (core, rim) = crate::progress::FAVOUR;
    commands.spawn((
        FavourGlyph,
        Sprite::from_image(images.add(crate::progress::glyph(core, rim))),
        Transform::default(),
        Visibility::Hidden,
        layer.clone(),
        corner(LENGTH + 12.0, 6.0, 5.1),
    ));
    commands.spawn((
        FavourCount,
        Text2d::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(8.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.86, 1.0)),
        Anchor::CENTER_LEFT,
        Transform::default(),
        Visibility::Hidden,
        layer.clone(),
        corner(LENGTH + 20.0, 6.0, 5.1),
    ));
    commands.spawn((
        Hint,
        Text2d::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(8.0),
            ..default()
        },
        TextColor(CREAM.with_alpha(0.8)),
        Anchor::TOP_LEFT,
        Transform::default(),
        Visibility::Hidden,
        layer,
        corner(0.0, 18.0, 5.1),
    ));
}

/// Keeps every piece at its place in the view's corner, on whole pixels.
fn follow(focus: Res<CameraFocus>, mut pieces: Query<(&At, &mut Transform)>) {
    let corner = focus.0.round() + Vec2::new(-(WIDTH as f32) / 2.0, HEIGHT as f32 / 2.0);
    for (at, mut t) in &mut pieces {
        t.translation = (corner + Vec2::new(at.0.x, -at.0.y)).extend(at.1);
    }
}

fn bars(
    players: Query<&Fighting, With<Player>>,
    ind: Res<Indulgence>,
    progress: Res<PlayerProgress>,
    mut bars: Query<(&Bar, &mut Transform, &mut Sprite)>,
) {
    let fighting = players.single().ok();
    for (bar, mut t, mut sprite) in &mut bars {
        let k = fighting.map_or(0.0, |f| match bar {
            Bar::Health => f.combat.health / f.max_health,
            Bar::Poise => f.combat.poise / f.combat.max_poise,
            Bar::Nectar => ind.nectar / progress.0.nectar_capacity(),
        });
        // Whole pixels, so the bar's end does not shimmer.
        t.scale.x = (LENGTH * k.clamp(0.0, 1.0)).round() / LENGTH;
        if let Bar::Nectar = bar {
            let color = if ind.nectar >= HIGH_COST {
                NECTAR_READY
            } else {
                NECTAR
            };
            if sprite.color != color {
                sprite.color = color;
            }
        }
    }
}

/// A pip for each Cordial that can be carried: lit while it is left, dim once drunk.
/// The one being drunk blinks, and every pip flashes when a tap finds none left.
fn pips(
    time: Res<Time<Real>>,
    ind: Res<Indulgence>,
    progress: Res<PlayerProgress>,
    mut pips: Query<(&Pip, &mut Sprite, &mut Visibility)>,
) {
    let carried = progress.0.cordials() as usize;
    let left = (ind.cordials as usize).min(carried);
    let blink = ((time.elapsed_secs() * 16.0) as u32).is_multiple_of(2);
    for (pip, mut sprite, mut vis) in &mut pips {
        vis.set_if_neq(if pip.0 < carried {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let color = if ind.empty > 0.0 && blink {
            CREAM
        } else if pip.0 < left || (pip.0 == left && ind.drinking.is_some() && blink) {
            CORDIAL
        } else {
            TRACK
        };
        if sprite.color != color {
            sprite.color = color;
        }
    }
}

/// The Favours not yet spent, and a pulse when one is gained.
fn favours(
    time: Res<Time<Real>>,
    progress: Res<PlayerProgress>,
    mut last: Local<Option<u32>>,
    mut pulse: Local<f32>,
    mut glyph: Query<(&mut Transform, &mut Visibility), With<FavourGlyph>>,
    mut count: Query<(&mut Text2d, &mut Visibility), (With<FavourCount>, Without<FavourGlyph>)>,
) {
    let n = progress.0.favours;
    if last.is_some_and(|l| n > l) {
        *pulse = 1.0;
    }
    *last = Some(n);
    *pulse = (*pulse - time.delta_secs()).max(0.0);
    let shown = if n > 0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Ok((mut t, mut vis)) = glyph.single_mut() {
        vis.set_if_neq(shown);
        // Whole steps: twice the size as the pulse starts, then back to one.
        t.scale = Vec3::splat(1.0 + pulse.ceil());
    }
    if let Ok((mut text, mut vis)) = count.single_mut() {
        vis.set_if_neq(shown);
        let s = format!("{n}");
        if text.0 != s {
            text.0 = s;
        }
    }
}

/// One line on what the left trigger would do now: drink when hurt, a High when one is affordable.
fn hint(
    players: Query<&Fighting, With<Player>>,
    ind: Res<Indulgence>,
    device: Res<Device>,
    mut hints: Query<(&mut Text2d, &mut Visibility), With<Hint>>,
) {
    let Ok((mut text, mut vis)) = hints.single_mut() else {
        return;
    };
    let key = Control::Indulge.key(*device);
    let hurt = players
        .single()
        .is_ok_and(|f| f.combat.health > 0.0 && f.combat.health < 0.5 * f.max_health);
    let high = matches!(ind.phase, Phase::High { .. });
    let line = if ind.drinking.is_some() {
        String::new()
    } else if hurt && ind.cordials > 0 {
        format!("{key} drink a Cordial")
    } else if hurt {
        "No Cordials left. Rest to refill".to_owned()
    } else if ind.nectar >= HIGH_COST && !high {
        format!("Hold {key} for a High")
    } else {
        String::new()
    };
    vis.set_if_neq(if line.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    });
    if text.0 != line {
        text.0 = line;
    }
}
