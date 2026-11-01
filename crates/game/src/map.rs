//! The map, opened with Select.
//!
//! It shows every room of the world the player has been in, drawn from the rooms' own tiles
//! at their places on the world map by `assetgen::map`, centred on the room the player is in,
//! with the player as a blinking dot. The game pauses while it is open.
//! In a test room outside the world there is no map; Select puts the player back at the spawn.

use std::collections::BTreeMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::input::TickInput;
use crate::player::Player;
use crate::progress::PlayerProgress;
use crate::render::{HEIGHT, ScreenCamera, WIDTH};
use crate::rooms::{ActiveRoom, RoomAsset};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_rooms)
            .add_systems(Update, (visit, toggle, redraw).chain());
    }
}

/// Every room of the world, loaded so the map can draw rooms the player is not in.
#[derive(Resource)]
struct WorldRooms(Vec<(String, Handle<RoomAsset>)>);

fn load_rooms(mut commands: Commands, assets: Res<AssetServer>) {
    let rooms = crate::rooms::world()
        .rooms
        .iter()
        .map(|r| {
            (
                r.name.clone(),
                assets.load(format!("rooms/{}.room", r.name)),
            )
        })
        .collect();
    commands.insert_resource(WorldRooms(rooms));
}

/// Notes each room of the world the player enters.
fn visit(active: Res<ActiveRoom>, mut progress: ResMut<PlayerProgress>) {
    if active.room.is_none() || progress.0.visited.contains(&active.name) {
        return;
    }
    if crate::rooms::world()
        .rooms
        .iter()
        .any(|r| r.name == active.name)
    {
        progress.0.visited.insert(active.name.clone());
    }
}

/// The open map: its image, and the blink it was last drawn with.
#[derive(Component)]
struct OpenMap {
    image: Handle<Image>,
    blink: Option<bool>,
}

fn toggle(
    mut commands: Commands,
    mut input: ResMut<TickInput>,
    active: Res<ActiveRoom>,
    mut time: ResMut<Time<Virtual>>,
    mut images: ResMut<Assets<Image>>,
    open: Query<Entity, With<OpenMap>>,
    screen: Query<Entity, With<ScreenCamera>>,
) {
    if !input.map_pressed {
        return;
    }
    input.map_pressed = false;
    if let Ok(e) = open.single() {
        commands.entity(e).despawn();
        time.unpause();
        return;
    }
    if !crate::rooms::world()
        .rooms
        .iter()
        .any(|r| r.name == active.name)
    {
        input.reset_pressed = true;
        return;
    }
    let Ok(camera) = screen.single() else { return };
    let image = images.add(Image::new_fill(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    commands.spawn((
        OpenMap {
            image: image.clone(),
            blink: None,
        },
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ImageNode::new(image),
        UiTargetCamera(camera),
        GlobalZIndex(10),
    ));
    time.pause();
}

fn redraw(
    real: Res<Time<Real>>,
    active: Res<ActiveRoom>,
    progress: Res<PlayerProgress>,
    world_rooms: Option<Res<WorldRooms>>,
    assets: Res<Assets<RoomAsset>>,
    players: Query<&Player>,
    mut maps: Query<&mut OpenMap>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Ok(mut map), Some(world_rooms)) = (maps.single_mut(), world_rooms) else {
        return;
    };
    let blink = ((real.elapsed_secs() * 3.0) as u32).is_multiple_of(2);
    if map.blink == Some(blink) {
        return;
    }
    map.blink = Some(blink);
    let rooms: BTreeMap<String, world::Room> = world_rooms
        .0
        .iter()
        .filter_map(|(name, h)| Some((name.clone(), (*assets.get(h)?.0).clone())))
        .collect();
    let world = crate::rooms::world();
    let player = players.single().ok().map(|p| {
        let b = &p.ctrl.body;
        (
            (b.x + b.w / 2).div_euclid(world::TILE),
            (b.y + b.h - 1).div_euclid(world::TILE),
        )
    });
    // Centre on the player, or on the world when they are elsewhere.
    let here = world.rooms.iter().find(|r| r.name == active.name);
    let centre = match (here.and_then(|r| r.pos), player) {
        (Some(pos), Some((x, y))) => (pos.0 + x, pos.1 + y),
        _ => {
            let b = assetgen::map::bounds(world, &rooms);
            ((b.0 + b.2) / 2, (b.1 + b.3) / 2)
        }
    };
    let view = assetgen::map::MapView {
        world,
        rooms: &rooms,
        shown: &progress.0.visited,
        current: Some(&active.name),
        player,
        dot: blink,
    };
    let img = assetgen::map::draw(&view, (WIDTH, HEIGHT), 2, centre);
    if let Some(mut image) = images.get_mut(&map.image) {
        image.data = Some(img.into_raw());
    }
}
