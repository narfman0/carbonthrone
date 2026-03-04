use bevy::prelude::*;
use carbonthrone::action_points::ActionPoints;
use carbonthrone::character::{Character, CharacterClass};
use carbonthrone::enemy::{Enemy, EnemyKind};
use carbonthrone::experience::Experience;
use carbonthrone::health::Health;
use carbonthrone::party::Party;
use carbonthrone::position::Position;
use carbonthrone::stats::Stats;

fn main() {
    let mut world = World::new();

    // Spawn character entities with all ECS components.
    // Characters start along the left edge of the grid (x=0), one row each.
    let mut party = Party::new();
    for (i, (name, class)) in [
        ("Aldric", CharacterClass::Warrior),
        ("Lyra",   CharacterClass::Rogue),
    ].into_iter().enumerate() {
        let stats = Stats::for_class(&class);
        let hp = stats.max_hp;
        let entity = world.spawn((
            Character::new(name, class),
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            Experience::new(),
            Position::new(0, i as i32, 0),
        )).id();
        party.add_member(entity).unwrap();
    }
    world.insert_resource(party);

    // Each enemy is an Entity carrying an Enemy component and a Position.
    world.spawn((Enemy::new(EnemyKind::Goblin, 1), Position::new(9, 0, 0)));
    world.spawn((Enemy::new(EnemyKind::Orc, 2),    Position::new(9, 1, 0)));

    // Query enemies with their positions.
    let mut enemy_query = world.query::<(&Enemy, &Position)>();
    for (e, pos) in enemy_query.iter(&world) {
        println!("Enemy: {} (hp {}) at ({},{},{})", e.name, e.current_hp, pos.x, pos.y, pos.z);
    }

    // Query characters with their positions.
    let mut char_query = world.query::<(&Character, &Health, &Position)>();
    for (c, h, pos) in char_query.iter(&world) {
        println!("Character: {} (hp {}/{}) at ({},{},{})", c.name, h.current, h.max, pos.x, pos.y, pos.z);
    }

    let party = world.resource::<Party>();
    println!("Party size: {}", party.size());
}
