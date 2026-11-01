//! Loading rooms from `.room` files and building them in the world.
//!
//! The rooms of the game are listed in `assets/world.ron`, which is built into the binary;
//! the feel lab and the combat lab are test rooms outside it.
//!
//! A room reloads while the game runs (with the `dev` feature): saving the file rebuilds
//! the room's tiles and lights in place, and the player stays where they are.

use std::sync::Arc;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::{ParseError, Room, World};

use crate::render::Light2d;

pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<RoomAsset>()
            .register_asset_loader(RoomLoader)
            .add_systems(Startup, load_first_room)
            .add_systems(PreUpdate, build_on_change)
            .add_systems(Update, (next_room, pulse));
    }
}

#[derive(Asset, TypePath)]
pub struct RoomAsset(pub Arc<Room>);

/// Test rooms outside the world, after its rooms in the order Start or Tab steps through them.
const TEST_ROOMS: [&str; 2] = ["lab", "arena"];

/// The world file, read once.
pub fn world() -> &'static World {
    static WORLD: std::sync::OnceLock<World> = std::sync::OnceLock::new();
    WORLD.get_or_init(|| {
        World::parse(include_str!("../../../assets/world.ron")).expect("assets/world.ron parses")
    })
}

/// Every room Start or Tab steps through: the world's, then the test rooms.
fn room_names() -> Vec<String> {
    let w = world();
    w.rooms
        .iter()
        .map(|r| r.name.clone())
        .chain(TEST_ROOMS.iter().map(|r| (*r).to_owned()))
        .collect()
}

/// The room the player is in. `room` is `None` until it has loaded.
#[derive(Resource)]
pub struct ActiveRoom {
    handle: Handle<RoomAsset>,
    /// The name of the room loading or loaded.
    pub name: String,
    pub room: Option<Arc<Room>>,
    /// Counts the times a room has been built. Things spawned from a room's entities
    /// are spawned once per build; a floor breaking changes the room but not the build.
    pub build: u64,
}

impl ActiveRoom {
    /// Starts loading another room. Its parts are rebuilt once it has loaded.
    pub fn go_to(&mut self, assets: &AssetServer, name: &str) {
        self.handle = assets.load(format!("rooms/{name}.room"));
        self.name = name.to_owned();
        self.room = None;
    }
}

/// A light that swells on every beat of the music: `pulse=0.6` in the room file.
#[derive(Component)]
struct Pulse {
    /// The light's strength in the room file.
    strength: f32,
    /// How deep the pulse is, 0 to 1.
    amount: f32,
}

/// Swells pulsing lights on the beat and lets them sink until the next.
fn pulse(beat: Res<crate::music::Beat>, mut lights: Query<(&Pulse, &mut Light2d)>) {
    let swell = (-beat.phase() * 6.0).exp();
    for (p, mut light) in &mut lights {
        light.strength = (p.strength * (1.0 - 0.5 * p.amount + p.amount * swell)).min(2.0);
    }
}

/// The sprite of the room's painted tiles.
#[derive(Component)]
pub struct RoomTiles;

/// Repaints the room's tiles, after a floor has broken.
pub fn repaint_tiles(room: &Room, images: &mut Assets<Image>) -> Handle<Image> {
    images.add(rasterize(room))
}

/// Marks everything built from the active room, so a rebuild can clear it.
#[derive(Component)]
pub struct RoomPart;

/// Starts in the room `PIXELVANIA_ROOM` names, or the world's start.
fn load_first_room(mut commands: Commands, assets: Res<AssetServer>) {
    let wanted = std::env::var("PIXELVANIA_ROOM").unwrap_or_default();
    let saved = crate::lounge::startup_save().map(|s| s.lounge.room);
    let name = room_names()
        .into_iter()
        .find(|r| *r == wanted)
        .or(saved)
        .unwrap_or_else(|| world().start.clone());
    commands.insert_resource(ActiveRoom {
        handle: assets.load(format!("rooms/{name}.room")),
        name,
        room: None,
        build: 0,
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
    let names = room_names();
    let at = names.iter().position(|r| *r == active.name).unwrap_or(0);
    let next = names[(at + 1) % names.len()].clone();
    active.go_to(&assets, &next);
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
    progress: Res<crate::progress::PlayerProgress>,
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
        // Floors broken earlier stay broken.
        let mut room = (*asset.0).clone();
        let broken: Vec<_> = progress.0.broken_in(&room.name).collect();
        for (x, y) in broken {
            room.clear(x, y);
        }
        let room = Arc::new(room);
        for part in &parts {
            commands.entity(part).despawn();
        }
        build(&mut commands, &mut images, &room);
        info!("built room `{}`", room.name);
        active.room = Some(room);
        active.build += 1;
    }
}

fn build(commands: &mut Commands, images: &mut Assets<Image>, room: &Room) {
    commands.spawn((
        RoomPart,
        RoomTiles,
        Name::new("tiles"),
        Sprite::from_image(images.add(rasterize(room))),
        Anchor::TOP_LEFT,
        Transform::default(),
    ));
    crate::hanging::spawn(commands, images, room);
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
        match read().and_then(|l| Ok((l, light.num("pulse", 0.0)?))) {
            Ok((l, pulse)) => {
                let (x, y) = light.pixel_pos();
                let strength = l.strength;
                let mut e = commands.spawn((
                    RoomPart,
                    Name::new("light"),
                    l,
                    Transform::from_xyz(x, -y, 0.0),
                ));
                if pulse > 0.0 {
                    e.insert(Pulse {
                        strength,
                        amount: pulse.min(1.0),
                    });
                }
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
