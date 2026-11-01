//! The Plunge breaking floors.
//!
//! A Plunge that lands on cracked stone shatters the whole patch of it, connected tile to
//! tile, and dives on through. The broken tiles are kept in the progress record, so the floor
//! stays broken when the player comes back, and the room's tiles are repainted at once.

use bevy::prelude::*;
use pixelfx::effects::EffectId;
use world::{CRACKED, TILE};

use crate::effects::SpawnFx;
use crate::player::Player;
use crate::progress::PlayerProgress;
use crate::rooms::{ActiveRoom, RoomTiles};

pub struct PlungePlugin;

impl Plugin for PlungePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlungeLanded>()
            .add_systems(Update, break_floors);
    }
}

/// A Plunge hit the ground, with the body's box in pixels: x, y, width, height.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlungeLanded {
    pub body: (i32, i32, i32, i32),
}

/// Breaks the cracked floor under each landed Plunge, and lets the Plunge carry on.
#[allow(clippy::too_many_arguments)]
fn break_floors(
    mut landed: MessageReader<PlungeLanded>,
    mut active: ResMut<ActiveRoom>,
    mut progress: ResMut<PlayerProgress>,
    mut players: Query<&mut Player>,
    mut tiles: Query<&mut Sprite, With<RoomTiles>>,
    mut images: ResMut<Assets<Image>>,
    mut fx: MessageWriter<SpawnFx>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
    mut shake: ResMut<crate::camera::Shake>,
) {
    for PlungeLanded { body: (x, y, w, h) } in landed.read().copied() {
        let Some(room) = &active.room else { continue };
        let row = (y + h).div_euclid(TILE);
        let start: Vec<(i32, i32)> = (x.div_euclid(TILE)..=(x + w - 1).div_euclid(TILE))
            .map(|tx| (tx, row))
            .filter(|&(tx, ty)| room.material(tx, ty) == Some(CRACKED))
            .collect();
        if start.is_empty() {
            continue;
        }
        // The whole connected patch goes.
        let mut patch = start.clone();
        let mut i = 0;
        while i < patch.len() {
            let (tx, ty) = patch[i];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (tx + dx, ty + dy);
                if room.material(n.0, n.1) == Some(CRACKED) && !patch.contains(&n) {
                    patch.push(n);
                }
            }
            i += 1;
        }
        let mut broken = (**room).clone();
        for &(tx, ty) in &patch {
            broken.clear(tx, ty);
            progress.0.broke(&room.name, tx, ty);
            let at = Vec2::new(
                (tx * TILE + TILE / 2) as f32,
                -((ty * TILE + TILE / 2) as f32),
            );
            fx.write(SpawnFx::new(EffectId::HeavyLanding, at));
        }
        for mut sprite in &mut tiles {
            sprite.image = crate::rooms::repaint_tiles(&broken, &mut images);
        }
        active.room = Some(std::sync::Arc::new(broken));
        for mut p in &mut players {
            p.ctrl.keep_plunging();
        }
        shake.kick(Vec2::NEG_Y, 6.0);
        sfx.write(crate::audio::PlaySfx::new(crate::audio::Sfx::WallSplat));
        info!("the Plunge broke {} tiles", patch.len());
    }
}
