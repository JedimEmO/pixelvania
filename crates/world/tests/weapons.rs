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
    assert!(count >= 8, "found only {count} weapon files");
}

#[test]
fn spinner_contacts_only_during_the_spin() {
    use world::combat::{AttackId, Attacker, Phase};
    let weapon =
        Weapon::from_ron(include_str!("../../../assets/combat/guest-spinner.ron")).unwrap();
    let mut attacker = Attacker::default();
    attacker.start(&weapon, AttackId::Combo1, 1);
    for tick in 0..weapon.combo[0].total_ticks() {
        let active = (36..180).contains(&tick);
        assert_eq!(attacker.phase() == Phase::Active, active);
        assert_eq!(!attacker.active_hitboxes((0.0, 0.0)).is_empty(), active);
        attacker.tick(&weapon, world::Controls::default(), true, 1, false);
    }
}

#[test]
fn toaster_has_no_body_attack() {
    let weapon =
        Weapon::from_ron(include_str!("../../../assets/combat/guest-toaster.ron")).unwrap();
    for id in world::combat::AttackId::ALL {
        assert!(weapon.attack(id).hitboxes.is_empty());
    }
}
