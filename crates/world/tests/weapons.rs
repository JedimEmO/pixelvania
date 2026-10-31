//! Every weapon file the game loads parses and passes validation.

use world::combat::Weapon;

#[test]
fn every_weapon_file_parses() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/combat");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "ron") {
            let text = std::fs::read_to_string(&path).unwrap();
            if let Err(e) = Weapon::from_ron(&text) {
                panic!("{}: {e}", path.display());
            }
            count += 1;
        }
    }
    assert!(count >= 5, "found only {count} weapon files");
}
