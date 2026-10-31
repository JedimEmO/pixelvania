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
use world::{ParseError, Room};

use crate::render::Light2d;

pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<RoomAsset>()
            .register_asset_loader(RoomLoader)
            .add_systems(Startup, load_first_room)
            .add_systems(PreUpdate, build_on_change)
            .add_systems(Update, next_room);
    }
}

#[derive(Asset, TypePath)]
pub struct RoomAsset(pub Arc<Room>);

/// The test rooms, in the order Start or Tab steps through them.
const ROOMS: [&str; 2] = ["lab", "arena"];

/// The room the player is in. `room` is `None` until it has loaded.
#[derive(Resource)]
pub struct ActiveRoom {
    handle: Handle<RoomAsset>,
    index: usize,
    pub room: Option<Arc<Room>>,
}

/// Marks everything built from the active room, so a rebuild can clear it.
#[derive(Component)]
pub struct RoomPart;

/// Starts in the room `PIXELVANIA_ROOM` names, or the feel lab.
fn load_first_room(mut commands: Commands, assets: Res<AssetServer>) {
    let wanted = std::env::var("PIXELVANIA_ROOM").unwrap_or_default();
    let index = ROOMS.iter().position(|r| *r == wanted).unwrap_or(0);
    commands.insert_resource(ActiveRoom {
        handle: assets.load(format!("rooms/{}.room", ROOMS[index])),
        index,
        room: None,
    });
}

/// Moves to the next test room. The player is despawned and respawns at its spawn point.
fn next_room(
    mut commands: Commands,
    input: Res<crate::input::TickInput>,
    assets: Res<AssetServer>,
    mut active: ResMut<ActiveRoom>,
    players: Query<Entity, With<crate::player::Player>>,
) {
    if !input.next_room_pressed {
        return;
    }
    active.index = (active.index + 1) % ROOMS.len();
    active.handle = assets.load(format!("rooms/{}.room", ROOMS[active.index]));
    active.room = None;
    for p in &players {
        commands.entity(p).despawn();
    }
}

fn build_on_change(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<RoomAsset>>,
    rooms: Res<Assets<RoomAsset>>,
    mut active: ResMut<ActiveRoom>,
    mut images: ResMut<Assets<Image>>,
    parts: Query<Entity, With<RoomPart>>,
) {
    // A room is built when it first loads, when its file changes,
    // and when the game switches back to a room that loaded earlier.
    let changed = events.read().any(|e| match e {
        AssetEvent::Added { id } | AssetEvent::Modified { id } => *id == active.handle.id(),
        _ => false,
    });
    if changed || active.room.is_none() {
        let id = active.handle.id();
        let Some(asset) = rooms.get(id) else { return };
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

/// Uploads the generated room albedo; alpha is the renderer's solid mask.
fn rasterize(room: &Room) -> Image {
    let pixels = assetgen::rasterize(room);
    Image::new(
        Extent3d {
            width: pixels.width(),
            height: pixels.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
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
