//! The reachability checker against the real world file and rooms.

use std::collections::{BTreeMap, BTreeSet};

use world::reach::{check_world, explore};
use world::{Ability, MoveTuning, Progress, Room, Side, TILE, Tile, World};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn load() -> (World, BTreeMap<String, Room>) {
    let world = World::parse(&std::fs::read_to_string(root().join("world.ron")).unwrap()).unwrap();
    let rooms = world
        .rooms
        .iter()
        .map(|r| {
            let src =
                std::fs::read_to_string(root().join(format!("rooms/{}.room", r.name))).unwrap();
            (r.name.clone(), Room::parse(&src).unwrap())
        })
        .collect();
    (world, rooms)
}

#[test]
fn the_ballroom_is_explored_from_its_door() {
    let (_, rooms) = load();
    let room = &rooms["ballroom"];
    let t = std::time::Instant::now();
    let reach = explore(
        room,
        &MoveTuning::default(),
        Progress::slice_start().movement(),
        (3.0 * TILE as f32, 37.0 * TILE as f32),
    );
    println!("{reach:?} in {:?}", t.elapsed());
    assert!(reach.doors.contains("west"));
    assert!(reach.lounges.contains("alcove"), "{reach:?}");
    assert!(reach.pickups.contains("candelabrum"), "{reach:?}");
}

/// The rooms under a cracked floor: entered only through a floor door with cracked tiles above it.
fn below_cracked_floors(rooms: &BTreeMap<String, Room>) -> BTreeSet<String> {
    let mut below = BTreeSet::new();
    for room in rooms.values() {
        let Some(cracked) = room.materials.iter().position(|m| m == "cracked") else {
            continue;
        };
        for d in room.doors().unwrap() {
            if d.side(room) != Some(Side::Bottom) {
                continue;
            }
            let (x0, x1) = (d.x as i32 / TILE, (d.x + d.w) as i32 / TILE);
            let under_cracks = (x0..x1).any(|x| {
                (0..room.height as i32).any(|y| room.tile(x, y) == Tile::Solid(cracked as u8))
            });
            if under_cracks {
                below.insert(d.to_room.clone());
            }
        }
    }
    below
}

#[test]
fn the_slice_reaches_everything_and_traps_nobody() {
    let (world, rooms) = load();
    let abilities = Progress::slice_start().abilities;
    let r = check_world(&world, &rooms, &MoveTuning::default(), &abilities);
    println!("{r:?}");
    // The Plunge breaks cracked floors, and the checker treats them as solid until it exists,
    // so the rooms below one (the cellar under the stage) are the only ones allowed to be missed.
    let below = below_cracked_floors(&rooms);
    let missed: BTreeSet<String> = world
        .rooms
        .iter()
        .map(|w| w.name.clone())
        .filter(|n| !r.rooms.contains(n))
        .collect();
    assert!(missed.is_subset(&below), "missed {missed:?}; {r:?}");
    assert!(r.traps.is_empty(), "{r:?}");
    assert!(r.pickups.contains("ballroom:candelabrum"));
    // Every pickup and lounge in a room reached is reached too.
    for name in &r.rooms {
        let room = &rooms[name];
        for p in room.entities_of("pickup") {
            let item = format!("{name}:{}", p.props["item"]);
            assert!(r.pickups.contains(&item), "{item} is not reached; {r:?}");
        }
        for l in room.entities_of("lounge") {
            let id = format!("{name}:{}", l.props["id"]);
            assert!(r.lounges.contains(&id), "{id} is not reached; {r:?}");
        }
    }
}

/// The ballroom's alcove, with its lounge and the candelabrum, is the Lure's to open.
#[test]
fn the_alcove_needs_the_lure() {
    let (world, rooms) = load();
    let mut abilities = Progress::slice_start().abilities;
    abilities.remove(&Ability::Lure);
    let r = check_world(&world, &rooms, &MoveTuning::default(), &abilities);
    assert!(!r.pickups.contains("ballroom:candelabrum"), "{r:?}");
    assert!(!r.lounges.contains("ballroom:alcove"), "{r:?}");
    assert!(r.traps.is_empty(), "{r:?}");
}
