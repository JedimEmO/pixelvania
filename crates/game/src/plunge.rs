//! The Plunge breaking floors.
//!
//! A Plunge that lands on cracked stone shatters the whole patch of it, connected tile to
//! tile, and dives on through. The broken tiles are kept in the progress record, so the floor
//! stays broken when the player comes back, and the room's tiles are repainted at once.
//!
//! The Plunge itself comes from the Dancer: a moment after the boss falls, the cracked stage
//! gives way under everyone, and the player drops through with the Plunge found.

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
            .add_systems(Update, (break_floors, stage_gives_way));
    }
}

/// A Plunge hit the ground, with the body's box in pixels: x, y, width, height.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlungeLanded {
    pub body: (i32, i32, i32, i32),
}

/// Everything a floor breaking touches.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Breaker<'w, 's> {
    active: ResMut<'w, ActiveRoom>,
    progress: ResMut<'w, PlayerProgress>,
    tiles: Query<'w, 's, &'static mut Sprite, With<RoomTiles>>,
    images: ResMut<'w, Assets<Image>>,
    fx: MessageWriter<'w, SpawnFx>,
    sfx: MessageWriter<'w, crate::audio::PlaySfx>,
    shake: ResMut<'w, crate::camera::Shake>,
}

impl Breaker<'_, '_> {
    /// Breaks every cracked tile connected to these, keeps the break in the progress record,
    /// and repaints the room. Returns how many tiles broke.
    pub fn shatter(&mut self, from: &[(i32, i32)]) -> usize {
        let Some(room) = self.active.room.clone() else {
            return 0;
        };
        let mut patch: Vec<(i32, i32)> = from
            .iter()
            .copied()
            .filter(|&(tx, ty)| room.material(tx, ty) == Some(CRACKED))
            .collect();
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
        if patch.is_empty() {
            return 0;
        }
        let mut broken = (*room).clone();
        for &(tx, ty) in &patch {
            broken.clear(tx, ty);
            self.progress.0.broke(&room.name, tx, ty);
            let at = Vec2::new(
                (tx * TILE + TILE / 2) as f32,
                -((ty * TILE + TILE / 2) as f32),
            );
            self.fx.write(SpawnFx::new(EffectId::HeavyLanding, at));
        }
        for mut sprite in &mut self.tiles {
            sprite.image = crate::rooms::repaint_tiles(&broken, &mut self.images);
        }
        self.active.room = Some(std::sync::Arc::new(broken));
        self.shake.kick(Vec2::NEG_Y, 6.0);
        self.sfx
            .write(crate::audio::PlaySfx::new(crate::audio::Sfx::WallSplat));
        patch.len()
    }

    /// Every cracked tile in the room.
    pub fn all_cracked(&self) -> Vec<(i32, i32)> {
        let Some(room) = &self.active.room else {
            return Vec::new();
        };
        (0..room.height as i32)
            .flat_map(|ty| (0..room.width as i32).map(move |tx| (tx, ty)))
            .filter(|&(tx, ty)| room.material(tx, ty) == Some(CRACKED))
            .collect()
    }
}

/// Breaks the cracked floor under each landed Plunge, and lets the Plunge carry on.
fn break_floors(
    mut landed: MessageReader<PlungeLanded>,
    mut breaker: Breaker,
    mut players: Query<&mut Player>,
) {
    for PlungeLanded { body: (x, y, w, h) } in landed.read().copied() {
        let row = (y + h).div_euclid(TILE);
        let under: Vec<(i32, i32)> = (x.div_euclid(TILE)..=(x + w - 1).div_euclid(TILE))
            .map(|tx| (tx, row))
            .collect();
        let n = breaker.shatter(&under);
        if n > 0 {
            for mut p in &mut players {
                p.ctrl.keep_plunging();
            }
            info!("the Plunge broke {n} tiles");
        }
    }
}

/// Seconds between the boss falling and the stage giving way, so the finisher has its moment.
const GIVE_WAY_AFTER: f32 = 1.6;

/// Favours the Dancer's fall is worth.
pub const DANCER_FAVOURS: u32 = 2;

/// After the Dancer falls, breaks the room's cracked floor and gives the Plunge, and two Favours.
fn stage_gives_way(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut defeated: MessageReader<crate::boss::BossDefeated>,
    mut waiting: Local<Option<f32>>,
    mut breaker: Breaker,
    device: Res<crate::prompt::Device>,
) {
    if defeated.read().count() > 0 {
        *waiting = Some(GIVE_WAY_AFTER);
    }
    let Some(left) = *waiting else { return };
    let left = left - time.delta_secs();
    if left > 0.0 {
        *waiting = Some(left);
        return;
    }
    *waiting = None;
    let cracked = breaker.all_cracked();
    let n = breaker.shatter(&cracked);
    let room = breaker.active.name.clone();
    let plunge = world::Item::Ability(world::Ability::Plunge);
    if breaker.progress.0.take(&room, plunge) {
        commands.spawn(crate::progress::Banner::bundle(plunge, *device));
        // The Host is pleased: two Favours, which the HUD shows arriving.
        breaker.progress.0.favours += DANCER_FAVOURS;
    }
    info!("the stage gave way: {n} tiles, and the Plunge is found");
}
