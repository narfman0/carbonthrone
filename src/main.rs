use bevy::prelude::*;
use carbonthrone::action_points::ActionPoints;
use carbonthrone::character::{Character, CharacterClass};
use carbonthrone::enemy::{Enemy, EnemyKind};
use carbonthrone::experience::Experience;
use carbonthrone::health::Health;
use carbonthrone::party::Party;
use carbonthrone::stats::Stats;

fn main() {
    let mut world = World::new();

    // Spawn character entities with all ECS components.
    let mut party = Party::new();
    for (name, class) in [
        ("Aldric", CharacterClass::Warrior),
        ("Lyra",   CharacterClass::Rogue),
    ] {
        let stats = Stats::for_class(&class);
        let hp = stats.max_hp;
        let entity = world.spawn((
            Character::new(name, class),
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            Experience::new(),
        )).id();
        party.add_member(entity).unwrap();
    }
    world.insert_resource(party);

    // Each enemy is an Entity carrying an Enemy component.
    world.spawn(Enemy::new(EnemyKind::Goblin, 1));
    world.spawn(Enemy::new(EnemyKind::Orc, 2));

    // Query enemies.
    let mut enemy_query = world.query::<&Enemy>();
    for e in enemy_query.iter(&world) {
        println!("Enemy: {} (hp {})", e.name, e.current_hp);
    }

    // Query character names and health.
    let mut char_query = world.query::<(&Character, &Health)>();
    for (c, h) in char_query.iter(&world) {
        println!("Character: {} (hp {}/{})", c.name, h.current, h.max);
    }

    let party = world.resource::<Party>();
    println!("Party size: {}", party.size());
}
