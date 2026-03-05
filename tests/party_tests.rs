use bevy::prelude::World;
use carbonthrone::character::Character;
use carbonthrone::health::Health;
use carbonthrone::npc::NPCKind;
use carbonthrone::party::{MAX_PARTY_SIZE, Party};

fn spawn_member(world: &mut World, name: &str) -> bevy::prelude::Entity {
    let ch = Character::new_player(name, NPCKind::Doss);
    let stats = ch.stats.clone();
    let hp = ch.current_hp;
    world.spawn((ch, stats, Health::new(hp))).id()
}

#[test]
fn can_add_up_to_five_members() {
    let mut world = World::new();
    let mut party = Party::new();
    for i in 0..5 {
        let e = spawn_member(&mut world, &format!("Hero{i}"));
        assert!(party.add_member(e).is_ok());
    }
    assert_eq!(party.size(), 5);
}

#[test]
fn cannot_exceed_max_party_size() {
    let mut world = World::new();
    let mut party = Party::new();
    for i in 0..MAX_PARTY_SIZE {
        let e = spawn_member(&mut world, &format!("Hero{i}"));
        party.add_member(e).unwrap();
    }
    let extra = spawn_member(&mut world, "Extra");
    assert!(party.add_member(extra).is_err());
}

#[test]
fn remove_member_works() {
    let mut world = World::new();
    let mut party = Party::new();
    let alice = spawn_member(&mut world, "Alice");
    let bob = spawn_member(&mut world, "Bob");
    party.add_member(alice).unwrap();
    party.add_member(bob).unwrap();
    let removed = party.remove_member(0).unwrap();
    assert_eq!(removed, alice);
    assert_eq!(party.size(), 1);
}

#[test]
fn is_wiped_when_all_dead() {
    let mut world = World::new();
    let mut party = Party::new();
    let e = spawn_member(&mut world, "Alice");
    party.add_member(e).unwrap();
    world.get_mut::<Health>(e).unwrap().take_damage(9999);
    assert!(party.is_wiped(&world));
}

#[test]
fn not_wiped_when_one_alive() {
    let mut world = World::new();
    let mut party = Party::new();
    let alice = spawn_member(&mut world, "Alice");
    let bob = spawn_member(&mut world, "Bob");
    party.add_member(alice).unwrap();
    party.add_member(bob).unwrap();
    world.get_mut::<Health>(alice).unwrap().take_damage(9999);
    assert!(!party.is_wiped(&world));
}
