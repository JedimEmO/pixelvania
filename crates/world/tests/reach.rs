//! The reachability checker against the real world file and rooms.

use std::collections::BTreeMap;

use world::reach::{check_world, explore};
use world::{Ability, MoveTuning, Progress, Room, TILE, World};

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

#[test]
fn the_slice_reaches_everything_and_traps_nobody() {
    let (world, rooms) = load();
    let abilities = Progress::slice_start().abilities;
    let r = check_world(&world, &rooms, &MoveTuning::default(), &abilities);
    println!("{r:?}");
    assert_eq!(r.rooms.len(), world.rooms.len(), "{r:?}");
    assert!(r.traps.is_empty(), "{r:?}");
    assert!(r.pickups.contains("ballroom:candelabrum"));
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
