use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    combat::{calc_damage, calc_hit_chance},
    health::Health,
    position::Position,
    side::Side,
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{Action, ATTACK_AP_COST, MOVE_AP_COST},
};

/// A fully-described action a player can choose for one of their combatants.
#[derive(Debug, Clone)]
pub enum PlayerActionChoice {
    /// Attack a living enemy.
    Attack {
        target: Entity,
        /// Probability of hitting, 0.0–1.0 (multiply by 100 for a percentage).
        hit_chance: f32,
        /// Damage dealt on a successful hit.
        damage: i32,
        /// Defender's cover level from this attack's direction.
        cover: CoverLevel,
    },
    /// Move toward a tile offering better cover than the current position.
    MoveToCover {
        destination: Position,
        /// Cover level the destination provides against the nearest enemy.
        cover: CoverLevel,
        /// Total AP cost (Manhattan distance × `MOVE_AP_COST`).
        ap_cost: i32,
    },
    /// End this actor's turn without spending more AP.
    Pass,
}

impl PlayerActionChoice {
    /// Convert to the low-level [`Action`] consumed by [`crate::turn::apply_action`].
    pub fn to_action(&self) -> Action {
        match self {
            Self::Attack { target, .. } => Action::Attack { target: *target },
            Self::MoveToCover { destination, .. } => Action::Move { destination: *destination },
            Self::Pass => Action::Pass,
        }
    }

    /// Short human-readable label suitable for a menu entry.
    pub fn display(&self) -> String {
        match self {
            Self::Attack { hit_chance, damage, cover, .. } => {
                let pct = (hit_chance * 100.0).round() as i32;
                match cover {
                    CoverLevel::None    => format!("Attack — {}% hit, {} dmg", pct, damage),
                    CoverLevel::Partial => format!("Attack — {}% hit, {} dmg (partial cover)", pct, damage),
                    CoverLevel::Full    => format!("Attack — {}% hit, {} dmg (full cover)", pct, damage),
                }
            }
            Self::MoveToCover { cover, ap_cost, .. } => {
                let label = match cover {
                    CoverLevel::None    => "open ground",
                    CoverLevel::Partial => "partial cover",
                    CoverLevel::Full    => "full cover",
                };
                format!("Move to {} (costs {} AP)", label, ap_cost)
            }
            Self::Pass => "Pass".to_string(),
        }
    }
}

/// Returns all valid actions available to `actor` this turn given their remaining AP.
///
/// Choices are grouped as follows:
/// 1. One [`PlayerActionChoice::Attack`] per living enemy, each annotated with hit
///    probability and expected damage (accounting for the defender's cover level).
/// 2. Up to one [`PlayerActionChoice::MoveToCover`] per reachable cover level that
///    is better than the actor's current cover, closest tile chosen for each level.
/// 3. [`PlayerActionChoice::Pass`] (always present as the last entry).
pub fn available_player_actions(world: &mut World, actor: Entity) -> Vec<PlayerActionChoice> {
    let mut choices = Vec::new();

    let ap = match world.get::<ActionPoints>(actor) {
        Some(ap) => ap.current,
        None => return choices,
    };
    let actor_pos = match world.get::<Position>(actor).copied() {
        Some(p) => p,
        None => return choices,
    };
    let actor_attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);

    // ── Attack options ─────────────────────────────────────────────────────────
    if ap >= ATTACK_AP_COST {
        // Collect target data before borrowing resources.
        let mut q = world.query::<(Entity, &Side, &Health, &Stats, &Position)>();
        let targets: Vec<(Entity, i32, i32, i32)> = q
            .iter(world)
            .filter(|(_, s, h, _, _)| **s == Side::Enemy && h.is_alive())
            .map(|(e, _, _, stats, pos)| (e, stats.defense, pos.x, pos.y))
            .collect();

        for (target_entity, defense, tx, ty) in targets {
            let dir = Direction::from_attack((actor_pos.x, actor_pos.y), (tx, ty));
            let cover = world
                .get_resource::<LevelMap>()
                .map(|m| m.get_cover(tx, ty, dir))
                .unwrap_or(CoverLevel::None);
            let hit_chance = calc_hit_chance(cover);
            let damage = calc_damage(actor_attack, defense);
            choices.push(PlayerActionChoice::Attack { target: target_entity, hit_chance, damage, cover });
        }
    }

    // ── Cover move options ─────────────────────────────────────────────────────
    let (cols, rows) = world
        .get_resource::<LevelMap>()
        .map(|m| (m.cols as i32, m.rows as i32))
        .unwrap_or((0, 0));

    if cols > 0 && rows > 0 {
        let mut q2 = world.query::<(&Side, &Health, &Position)>();
        let enemy_positions: Vec<(i32, i32)> = q2
            .iter(world)
            .filter(|(s, h, _)| **s == Side::Enemy && h.is_alive())
            .map(|(_, _, pos)| (pos.x, pos.y))
            .collect();

        if let Some(&(ex, ey)) = enemy_positions
            .iter()
            .min_by_key(|(ex, ey)| (ex - actor_pos.x).abs() + (ey - actor_pos.y).abs())
        {
            let attack_dir = Direction::from_attack((ex, ey), (actor_pos.x, actor_pos.y));
            let current_cover = world
                .get_resource::<LevelMap>()
                .map(|m| m.get_cover(actor_pos.x, actor_pos.y, attack_dir))
                .unwrap_or(CoverLevel::None);

            // Collect candidate cover tiles within AP budget.
            let mut candidates: Vec<(CoverLevel, i32, i32, i32)> = Vec::new();
            if let Some(map) = world.get_resource::<LevelMap>() {
                for dy in -ap..=ap {
                    for dx in -ap..=ap {
                        let dist = dx.abs() + dy.abs();
                        let cost = dist * MOVE_AP_COST;
                        if dist == 0 || cost > ap {
                            continue;
                        }
                        let tx = actor_pos.x + dx;
                        let ty = actor_pos.y + dy;
                        if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
                            continue;
                        }
                        if !map.is_passable(tx, ty) {
                            continue;
                        }
                        let tile_cover = map.get_cover(tx, ty, attack_dir);
                        if tile_cover > current_cover {
                            candidates.push((tile_cover, dist, tx, ty));
                        }
                    }
                }
            }

            // Best cover first, then closest. Emit one entry per cover level.
            candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut seen: HashSet<u8> = HashSet::new();
            for (tile_cover, dist, tx, ty) in candidates {
                if seen.insert(tile_cover as u8) {
                    choices.push(PlayerActionChoice::MoveToCover {
                        destination: Position::new(tx, ty, actor_pos.z),
                        cover: tile_cover,
                        ap_cost: dist * MOVE_AP_COST,
                    });
                }
            }
        }
    }

    choices.push(PlayerActionChoice::Pass);
    choices
}
