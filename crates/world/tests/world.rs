//! The world file and the doors between its rooms.

use world::{Room, World};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

#[test]
fn every_door_leads_to_a_door_that_leads_back() {
    let world = World::parse(&std::fs::read_to_string(root().join("world.ron")).unwrap()).unwrap();
    let problems = world.check(|name| {
        let src = std::fs::read_to_string(root().join(format!("rooms/{name}.room")))
            .map_err(|e| e.to_string())?;
        Room::parse(&src).map_err(|e| e.to_string())
    });
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
