use bevy::prelude::*;
use carbonthrone::character::{Character, CharacterClass};
use carbonthrone::enemy::{Enemy, EnemyKind};
use carbonthrone::party::Party;

fn main() {
    let mut world = World::new();

    // Party is a global Resource — one party exists for the whole game.
    let mut party = Party::new();
    party.add_member(Character::new("Aldric", CharacterClass::Warrior)).unwrap();
    party.add_member(Character::new("Lyra", CharacterClass::Mage)).unwrap();
    world.insert_resource(party);

    // Each enemy is an Entity carrying an Enemy component.
    world.spawn(Enemy::new(EnemyKind::Goblin, 1));
    world.spawn(Enemy::new(EnemyKind::Orc, 2));

    // Query all living enemies to verify ECS integration.
    let mut enemy_query = world.query::<&Enemy>();
    for e in enemy_query.iter(&world) {
        println!("Enemy: {} (hp {})", e.name, e.current_hp);
    }

    // Access the party resource.
    let party = world.resource::<Party>();
    println!("Party size: {}", party.size());
}
