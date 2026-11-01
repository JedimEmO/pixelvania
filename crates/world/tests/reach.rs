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

/// The movement abilities found in the slice before the Dancer: Lunge, Cling, Swoon and Lure.
fn before_the_dancer() -> BTreeSet<Ability> {
    [
        Ability::Lunge,
        Ability::Cling,
        Ability::Swoon,
        Ability::Lure,
    ]
    .into()
}

/// A new game, with these abilities found.
fn with(abilities: &BTreeSet<Ability>) -> Progress {
    Progress {
        abilities: abilities.clone(),
        ..Progress::new_game()
    }
}

#[test]
fn the_ballroom_is_explored_from_its_door() {
    let (_, rooms) = load();
    let room = &rooms["ballroom"];
    let t = std::time::Instant::now();
    let reach = explore(
        room,
        &MoveTuning::default(),
        with(&before_the_dancer()).movement(),
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
    let abilities = before_the_dancer();
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
    let mut abilities = before_the_dancer();
    abilities.remove(&Ability::Lure);
    let r = check_world(&world, &rooms, &MoveTuning::default(), &abilities);
    assert!(!r.pickups.contains("ballroom:candelabrum"), "{r:?}");
    assert!(!r.lounges.contains("ballroom:alcove"), "{r:?}");
    assert!(r.traps.is_empty(), "{r:?}");
}

/// With the Plunge, the floor under the stage opens and the slice can be finished.
#[test]
fn with_the_plunge_every_room_is_reached() {
    let (world, rooms) = load();
    let mut abilities = before_the_dancer();
    abilities.insert(Ability::Plunge);
    let r = check_world(&world, &rooms, &MoveTuning::default(), &abilities);
    assert_eq!(r.rooms.len(), world.rooms.len(), "{r:?}");
    assert!(r.traps.is_empty(), "{r:?}");
}

/// A new game starts with nothing in the Arrival, and the night hands out its abilities in order:
/// each step's pickups are reached with what was found before them and not sooner,
/// every step opens more rooms, and no step leaves a way in without a way out.
#[test]
fn the_slice_gives_its_abilities_in_order() {
    let (world, rooms) = load();
    let t = MoveTuning::default();
    // What is found with the abilities so far, and the ability found next.
    let order: [(&[&str], Ability); 5] = [
        // The Arrival: the Lunge in the coat check, Cling at the bottom of the coat lift.
        (&["coat_check:lunge"], Ability::Lunge),
        (&["coat_lift:cling", "porters_lodge:favour"], Ability::Cling),
        // Up the grand stair, the feast: the Jeroboam on the Glutton's table, the Swoon after him.
        // The ballroom's wing, past the Doorman, is open too.
        (
            &[
                "gluttons_hall:jeroboam",
                "afters:swoon",
                "kitchens:goblet",
                "letter_room:favour",
                "champagne_cellar:favour",
            ],
            Ability::Swoon,
        ),
        (&["crystal_gallery:lure", "afters:favour"], Ability::Lure),
        // The Dancer gives the Plunge, after the candelabrum.
        (&["ballroom:candelabrum"], Ability::Plunge),
    ];
    let mut found = Progress::new_game().abilities;
    assert!(found.is_empty(), "a new game starts with no abilities");
    let mut rooms_before = 0;
    for (i, (pickups, next)) in order.iter().enumerate() {
        let r = check_world(&world, &rooms, &t, &found);
        println!("with {found:?}: {} rooms; {:?}", r.rooms.len(), r.pickups);
        assert!(r.traps.is_empty(), "with {found:?}: {:?}", r.traps);
        assert!(
            r.rooms.len() > rooms_before,
            "with {found:?} no new rooms open: {:?}",
            r.rooms
        );
        rooms_before = r.rooms.len();
        for p in *pickups {
            assert!(r.pickups.contains(*p), "with {found:?}, {p} is not reached");
        }
        if let Some((later, _)) = order.get(i + 1) {
            for p in *later {
                assert!(
                    !r.pickups.contains(*p),
                    "with {found:?}, {p} is reached too soon"
                );
            }
        }
        found.insert(*next);
    }
    // With the Plunge, the floor under the stage gives way and every room is reached.
    let r = check_world(&world, &rooms, &t, &found);
    assert!(r.traps.is_empty(), "{:?}", r.traps);
    assert_eq!(r.rooms.len(), world.rooms.len(), "{:?}", r.rooms);
}

/// With the Lure, everything is open but the cellar under the stage.
#[test]
fn the_lure_opens_everything_but_the_cellar() {
    let (world, rooms) = load();
    let r = check_world(&world, &rooms, &MoveTuning::default(), &before_the_dancer());
    let missed: Vec<&str> = world
        .rooms
        .iter()
        .map(|w| w.name.as_str())
        .filter(|n| !r.rooms.contains(*n))
        .collect();
    assert_eq!(missed, ["cellar"], "{r:?}");
}
