use bevy::prelude::*;

use crate::position::Position;
use crate::terrain::LevelMap;
use crate::zone::{CardinalDir, Zone};

/// Move each companion one step toward the player if Chebyshev distance > 3.
pub fn follow_companions(
    world: &mut World,
    player_entity: Entity,
    companion_entities: &[Entity],
    zone: &Zone,
) {
    let player_pos = match world.get::<Position>(player_entity) {
        Some(p) => *p,
        None => return,
    };
    for &entity in companion_entities {
        let comp_pos = match world.get::<Position>(entity) {
            Some(p) => *p,
            None => continue,
        };
        let dx = player_pos.x - comp_pos.x;
        let dy = player_pos.y - comp_pos.y;
        let chebyshev = dx.abs().max(dy.abs());
        if chebyshev <= 3 {
            continue;
        }
        let sx = dx.signum();
        let sy = dy.signum();
        for (cx, cy) in [(sx, sy), (sx, 0), (0, sy)] {
            if cx == 0 && cy == 0 {
                continue;
            }
            let nx = (comp_pos.x + cx).clamp(0, zone.cols as i32 - 1);
            let ny = (comp_pos.y + cy).clamp(0, zone.rows as i32 - 1);
            if zone.map.get(nx, ny).is_passable() {
                if let Some(mut pos) = world.get_mut::<Position>(entity) {
                    *pos = Position::new(nx, ny);
                }
                break;
            }
        }
    }
}

/// Teleport each companion to a position adjacent to `spawn` within zone bounds.
///
/// Each companion is offset by `(i+1, 0)` from `spawn`, then snapped to the
/// nearest passable tile using `map`.
pub fn place_companions_near(
    world: &mut World,
    companion_entities: &[Entity],
    spawn: (i32, i32),
    map: &LevelMap,
) {
    let cols = map.cols as i32;
    let rows = map.rows as i32;
    for (i, &entity) in companion_entities.iter().enumerate() {
        let ox = i as i32 + 1;
        let cx = (spawn.0 + ox).clamp(0, cols - 1);
        let cy = spawn.1.clamp(0, rows - 1);
        let (nx, ny) = map.nearest_open_tile(cx, cy);
        if let Some(mut pos) = world.get_mut::<Position>(entity) {
            *pos = Position::new(nx, ny);
        }
    }
}

/// Returns the position 1 tile inward from the door on `door_dir` side of `zone`.
/// Used to place the player just inside a zone after transitioning.
pub fn spawn_pos_near_door(zone: &Zone, door_dir: CardinalDir) -> (i32, i32) {
    let door_pos = zone
        .doors
        .iter()
        .find(|entry| *entry.1 == door_dir)
        .map(|entry| *entry.0);

    let Some((x, y)) = door_pos else {
        return (1, 1);
    };

    let nx = match door_dir {
        CardinalDir::East => (x - 1).max(0),
        CardinalDir::West => (x + 1).min(zone.cols as i32 - 1),
        _ => x,
    };
    let ny = match door_dir {
        CardinalDir::North => (y + 1).min(zone.rows as i32 - 1),
        CardinalDir::South => (y - 1).max(0),
        _ => y,
    };
    zone.map.nearest_open_tile(nx, ny)
}
