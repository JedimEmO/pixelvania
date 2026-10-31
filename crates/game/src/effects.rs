//! pixelfx effects in the room: simulated and painted on the CPU, then composited into the buffers.
//!
//! An effect's colour is a sprite in the colour buffer, so the scene's lights fall on its dust
//! and debris. Its own light goes two ways: the painted light into the glow buffer, where it
//! burns hot, and a point light at its origin whose strength follows how much light the effect
//! gives off this frame, so a slash lights the walls around it.
//!
//! Particles land on the room's floors through pixelfx's ground query. The query only knows
//! floors, so colour that falls inside a solid tile is hidden: sparks go behind a wall, not over it.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use pixelfx::{Effect, EffectId, Frame, Options};
use world::{Room, TILE};

use crate::render::{AdditiveImage, GLOW_LAYER, Light2d};
use crate::rooms::ActiveRoom;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFx>().add_systems(
            Update,
            (spawn_fx, run_fx).chain().after(crate::player::react),
        );
    }
}

/// Asks for an effect at a point in the room.
#[derive(Message, Clone)]
pub struct SpawnFx {
    pub id: EffectId,
    /// The effect's origin in world space (y up). For most effects, the ground point under it.
    pub pos: Vec2,
    pub options: Options,
}

impl SpawnFx {
    pub fn new(id: EffectId, pos: Vec2) -> Self {
        Self {
            id,
            pos,
            options: Options::default(),
        }
    }

    /// Mirrors the effect, for one that faces left.
    pub fn facing(mut self, facing: i32) -> Self {
        self.options.flip = facing < 0;
        self
    }

    pub fn dust(pos: Vec2, variant: pixelfx::effects::DustVariant) -> Self {
        let mut fx = Self::new(EffectId::Dust, pos);
        fx.options.dust = variant;
        fx
    }
}

/// Effect light per unit of painted light: the sum over the light buffer of each pixel's
/// brightness, 0 to 1, that makes the point light full strength.
const LIGHT_PER_STRENGTH: f32 = 220.0;
const MAX_LIGHT_STRENGTH: f32 = 1.6;

#[derive(Component)]
struct Fx {
    effect: Effect,
    frame: Frame,
    /// The pixel of the frame the effect is drawn about.
    origin: [f64; 2],
    /// Where `origin` is in the room, in pixels with y down.
    at: [f64; 2],
    ground: RoomGround,
    color: Handle<Image>,
    light: Handle<Image>,
}

/// Floor heights under an effect, column by column, taken from the room's tiles once at spawn.
struct RoomGround {
    room: Arc<Room>,
    /// World x of `floors[0]`.
    x0: i32,
    /// Floor y per column, y down, or infinity where there is none.
    floors: Vec<f64>,
}

impl RoomGround {
    fn new(room: Arc<Room>, at: [f64; 2], half_width: i32) -> Self {
        let x0 = at[0].floor() as i32 - half_width;
        // A floor counts if its top is at most a tile above the origin, so effects on a floor
        // find it, and particles thrown up a little can still land on a step.
        let from_row = (at[1].floor() as i32 - TILE).div_euclid(TILE);
        let (_, room_h) = room.pixel_size();
        let rows = room_h.div_euclid(TILE);
        let floors = (0..half_width * 2)
            .map(|i| {
                let tx = (x0 + i).div_euclid(TILE);
                (from_row.max(0)..rows)
                    .find(|&ty| room.is_solid(tx, ty) && !room.is_solid(tx, ty - 1))
                    .map_or(f64::INFINITY, |ty| (ty * TILE) as f64)
            })
            .collect();
        Self { room, x0, floors }
    }

    /// Floor height under a world x, y down.
    fn floor_at(&self, x: f64) -> f64 {
        let i = x.floor() as i32 - self.x0;
        usize::try_from(i)
            .ok()
            .and_then(|i| self.floors.get(i).copied())
            .unwrap_or(f64::INFINITY)
    }
}

fn new_image(width: u32, height: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn spawn_fx(
    mut commands: Commands,
    mut requests: MessageReader<SpawnFx>,
    active: Res<ActiveRoom>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<AdditiveImage>>,
    mut seed: Local<u32>,
) {
    let Some(room) = &active.room else {
        requests.clear();
        return;
    };
    for req in requests.read() {
        *seed = seed.wrapping_add(1);
        let meta = req.id.metadata();
        let [w, h] = meta.size;
        // A mirrored effect extends the other way from its origin, so the frame mirrors too.
        let origin = if req.options.flip {
            [w as f64 - meta.origin[0], meta.origin[1]]
        } else {
            meta.origin
        };
        let pos = req.pos.round();
        let at = [pos.x as f64, -pos.y as f64];
        let color = images.add(new_image(w, h));
        let light = images.add(new_image(w, h));
        // Offset of the frame's centre from its origin, in world space.
        let centre = Vec2::new(
            w as f32 / 2.0 - origin[0] as f32,
            origin[1] as f32 - h as f32 / 2.0,
        );
        commands.spawn((
            Name::new(format!("fx {}", req.id.name())),
            Fx {
                effect: Effect::new(req.id, *seed, req.options.clone()),
                frame: Frame::new(w, h),
                origin,
                at,
                ground: RoomGround::new(room.clone(), at, w as i32),
                color: color.clone(),
                light: light.clone(),
            },
            Transform::from_xyz(pos.x, pos.y, 2.0),
            Visibility::default(),
            Light2d {
                color: LinearRgba::WHITE,
                radius: (w.max(h) as f32 * 0.9).max(64.0),
                strength: 0.0,
                glow: 0.0,
            },
            children![
                (
                    Sprite::from_image(color),
                    Anchor(Vec2::new(
                        origin[0] as f32 / w as f32 - 0.5,
                        0.5 - origin[1] as f32 / h as f32,
                    )),
                ),
                (
                    Mesh2d(meshes.add(Rectangle::new(w as f32, h as f32))),
                    MeshMaterial2d(materials.add(AdditiveImage { image: light })),
                    Transform::from_translation(centre.extend(0.0)),
                    RenderLayers::layer(GLOW_LAYER),
                ),
            ],
        ));
    }
}

fn run_fx(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut Fx, &mut Light2d)>,
    mut images: ResMut<Assets<Image>>,
) {
    let dt = time.delta_secs_f64();
    for (entity, mut fx, mut light) in &mut effects {
        let fx = &mut *fx;
        let [ax, ay] = fx.at;
        let flip = if fx.effect.options.flip { -1.0 } else { 1.0 };
        let ground = &fx.ground;
        fx.effect
            .update(dt, &|x: f64| ground.floor_at(ax + x * flip) - ay);
        if fx.effect.finished() {
            commands.entity(entity).despawn();
            continue;
        }
        fx.frame.clear();
        fx.effect.draw(&mut fx.frame, fx.origin);
        hide_inside_walls(fx);

        let (energy, sum) = fx.frame.light.iter().fold((0.0, Vec3::ZERO), |(e, s), p| {
            let c = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0;
            (e + c.max_element(), s + c)
        });
        let strength = (energy / LIGHT_PER_STRENGTH).min(MAX_LIGHT_STRENGTH);
        let hue = sum / sum.max_element().max(1e-6);
        let color = LinearRgba::rgb(hue.x, hue.y, hue.z);
        if (light.strength - strength).abs() > 0.01 || light.color != color {
            light.strength = strength;
            light.color = color;
        }

        if let Some(mut img) = images.get_mut(&fx.color) {
            img.data = Some(fx.frame.color.as_flattened().to_vec());
        }
        if let Some(mut img) = images.get_mut(&fx.light) {
            img.data = Some(fx.frame.light.as_flattened().to_vec());
        }
    }
}

/// Clears colour that falls inside solid tiles. Light is left, so it still spills onto walls.
fn hide_inside_walls(fx: &mut Fx) {
    let w = fx.frame.width as i32;
    let left = fx.at[0] as i32 - fx.origin[0] as i32;
    let top = fx.at[1] as i32 - fx.origin[1] as i32;
    let room = &fx.ground.room;
    for (i, px) in fx.frame.color.iter_mut().enumerate() {
        if px[3] == 0 {
            continue;
        }
        let x = left + i as i32 % w;
        let y = top + i as i32 / w;
        if room.is_solid(x.div_euclid(TILE), y.div_euclid(TILE)) {
            *px = [0; 4];
        }
    }
}
