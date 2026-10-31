//! Loading rooms from `.room` files and building them in the world.
//!
//! A room reloads while the game runs (with the `dev` feature): saving the file rebuilds
//! the room's tiles and lights in place, and the player stays where they are.

use std::sync::Arc;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::{ParseError, Room, TILE, Tile};

use crate::render::Light2d;

pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<RoomAsset>()
            .register_asset_loader(RoomLoader)
            .add_systems(Startup, load_first_room)
            .add_systems(PreUpdate, build_on_change);
    }
}

#[derive(Asset, TypePath)]
pub struct RoomAsset(pub Arc<Room>);

/// The room the player is in. `room` is `None` until it has loaded.
#[derive(Resource)]
pub struct ActiveRoom {
    handle: Handle<RoomAsset>,
    pub room: Option<Arc<Room>>,
}

/// Marks everything built from the active room, so a rebuild can clear it.
#[derive(Component)]
struct RoomPart;

fn load_first_room(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ActiveRoom {
        handle: assets.load("rooms/lab.room"),
        room: None,
    });
}

fn build_on_change(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<RoomAsset>>,
    rooms: Res<Assets<RoomAsset>>,
    mut active: ResMut<ActiveRoom>,
    mut images: ResMut<Assets<Image>>,
    parts: Query<Entity, With<RoomPart>>,
) {
    for event in events.read() {
        let id = match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => *id,
            _ => continue,
        };
        if id != active.handle.id() {
            continue;
        }
        let Some(asset) = rooms.get(id) else { continue };
        let room = asset.0.clone();
        for part in &parts {
            commands.entity(part).despawn();
        }
        build(&mut commands, &mut images, &room);
        info!("built room `{}`", room.name);
        active.room = Some(room);
    }
}

fn build(commands: &mut Commands, images: &mut Assets<Image>, room: &Room) {
    commands.spawn((
        RoomPart,
        Name::new("tiles"),
        Sprite::from_image(images.add(rasterize(room))),
        Anchor::TOP_LEFT,
        Transform::default(),
    ));
    for light in room.entities_of("light") {
        let read = || -> Result<Light2d, ParseError> {
            let [r, g, b] = light.color("color", [1.0, 0.9, 0.7])?;
            Ok(Light2d {
                color: Color::srgb(r, g, b).to_linear(),
                radius: light.num("radius", 96.0)?,
                strength: light.num("strength", 1.0)?,
                glow: light.num("glow", 0.0)?,
            })
        };
        match read() {
            Ok(l) => {
                let (x, y) = light.pixel_pos();
                commands.spawn((
                    RoomPart,
                    Name::new("light"),
                    l,
                    Transform::from_xyz(x, -y, 0.0),
                ));
            }
            Err(e) => error!("room `{}`: {e}", room.name),
        }
    }
}

/// Flat colours per material, as sRGB: a base tone and a slightly darker one.
fn material_tones(name: &str) -> ([u8; 3], [u8; 3]) {
    match name {
        "stone" => ([0x3a, 0x35, 0x46], [0x33, 0x2f, 0x3e]),
        "wood" => ([0x4a, 0x34, 0x28], [0x41, 0x2d, 0x23]),
        _ => ([0xff, 0x00, 0xff], [0xc0, 0x00, 0xc0]),
    }
}

/// Draws the room's tiles into one image, one texel per game pixel.
/// Empty space is transparent, which is how the renderer tells solid from empty.
fn rasterize(room: &Room) -> Image {
    let (w, h) = room.pixel_size();
    let (w, h) = (w as usize, h as usize);
    let tones: Vec<_> = room.materials.iter().map(|m| material_tones(m)).collect();
    let mut data = vec![0u8; w * h * 4];
    for py in 0..h {
        for px in 0..w {
            let (tx, ty) = (px as i32 / TILE, py as i32 / TILE);
            let Tile::Solid(m) = room.tile(tx, ty) else {
                continue;
            };
            let (base, dark) = tones[m as usize];
            // A quiet two-tone texture, so large walls are not perfectly flat.
            let rgb = if hash(px as u32 / 2, py as u32 / 2).is_multiple_of(5) {
                dark
            } else {
                base
            };
            let i = (py * w + px) * 4;
            data[i..i + 3].copy_from_slice(&rgb);
            data[i + 3] = 255;
        }
    }
    Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

#[derive(TypePath, Default)]
struct RoomLoader;

impl AssetLoader for RoomLoader {
    type Asset = RoomAsset;
    type Settings = ();
    type Error = RoomLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut LoadContext<'_>,
    ) -> Result<RoomAsset, RoomLoadError> {
        let mut text = String::new();
        reader
            .read_to_string(&mut text)
            .await
            .map_err(RoomLoadError::Io)?;
        Ok(RoomAsset(Arc::new(
            Room::parse(&text).map_err(RoomLoadError::Parse)?,
        )))
    }

    fn extensions(&self) -> &[&str] {
        &["room"]
    }
}

#[derive(Debug)]
enum RoomLoadError {
    Io(std::io::Error),
    Parse(ParseError),
}

impl std::fmt::Display for RoomLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomLoadError::Io(e) => write!(f, "reading room: {e}"),
            RoomLoadError::Parse(e) => write!(f, "parsing room: {e}"),
        }
    }
}

impl std::error::Error for RoomLoadError {}
