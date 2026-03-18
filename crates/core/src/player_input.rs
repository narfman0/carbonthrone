use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    ability::{Ability, AbilityEffect, AbilityKind, available_abilities},
    action_points::ActionPoints,
    character::{Aggression, Character},
    combat::{calc_damage, calc_hit_chance},
    health::Health,
    position::Position,
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{Action, move_range_per_ap},
};

/// A fully-described action a player can choose for one of their combatants.
#[derive(Debug, Clone)]
pub enum PlayerActionChoice {
    /// Use an ability — either offensive (targeting a living enemy) or utility (self/no target).
    UseAbility {
        ability: Ability,
        target: Option<Entity>,
        /// Pre-computed hit chance for offensive abilities; `None` for utility abilities.
        hit_chance: Option<f32>,
        /// Pre-computed damage for damage-dealing abilities; `None` for utility/disruption.
        damage: Option<i32>,
        /// Defender's cover level from this attack direction; `None` for utility or no-cover info.
        cover: Option<CoverLevel>,
    },
    /// Move toward a tile offering better cover than the current position.
    MoveToCover {
        destination: Position,
        /// Cover level the destination provides against the nearest enemy.
        cover: CoverLevel,
        /// Total AP cost (BFS path length converted to AP via speed stat).
        ap_cost: i32,
    },
    /// Move to an arbitrary passable tile (initiated by right-click on map).
    Move {
        destination: Position,
        /// Total AP cost for the move.
        ap_cost: i32,
    },
    /// End this actor's turn without spending more AP.
    Pass,
}

impl PlayerActionChoice {
    /// Convert to the low-level [`Action`] consumed by [`crate::turn::apply_action`].
    pub fn to_action(&self) -> Action {
        match self {
            Self::UseAbility {
                ability, target, ..
            } => Action::UseAbility {
                ability: ability.clone(),
                target: *target,
            },
            Self::MoveToCover { destination, .. } | Self::Move { destination, .. } => {
                Action::Move {
                    destination: *destination,
                }
            }
            Self::Pass => Action::Pass,
        }
    }

    /// Short human-readable label suitable for a menu entry.
    pub fn display(&self) -> String {
        match self {
            Self::UseAbility {
                ability,
                hit_chance,
                damage,
                cover,
                ..
            } => match (hit_chance, damage) {
                (Some(hc), Some(dmg)) => {
                    let pct = (hc * 100.0).round() as i32;
                    match cover.unwrap_or(CoverLevel::None) {
                        CoverLevel::None => {
                            format!(
                                "{} — {}% hit, {} dmg ({}AP)",
                                ability.name, pct, dmg, ability.ap_cost
                            )
                        }
                        CoverLevel::Partial => {
                            format!(
                                "{} — {}% hit, {} dmg, partial cover ({}AP)",
                                ability.name, pct, dmg, ability.ap_cost
                            )
                        }
                        CoverLevel::Full => {
                            format!(
                                "{} — {}% hit, {} dmg, full cover ({}AP)",
                                ability.name, pct, dmg, ability.ap_cost
                            )
                        }
                    }
                }
                _ => format!("{} ({}AP)", ability.name, ability.ap_cost),
            },
            Self::MoveToCover { cover, ap_cost, .. } => {
                let label = match cover {
                    CoverLevel::None => "open ground",
                    CoverLevel::Partial => "partial cover",
                    CoverLevel::Full => "full cover",
                };
                format!("Move to {} (costs {} AP)", label, ap_cost)
            }
            Self::Move {
                destination,
                ap_cost,
            } => {
                format!(
                    "Move to ({},{}) ({} AP)",
                    destination.x, destination.y, ap_cost
                )
            }
            Self::Pass => "Pass".to_string(),
        }
    }
}

/// Returns all valid actions available to `actor` this turn given their remaining AP.
///
/// Choices are:
/// 1. [`PlayerActionChoice::UseAbility`] for each offensive ability × each valid target:
///    - Ranged abilities: one entry per living enemy.
///    - Melee abilities: one entry per adjacent living enemy (Chebyshev distance ≤ 1,
///      diagonals included).
/// 2. [`PlayerActionChoice::UseAbility`] for each utility ability (self-targeted, once each).
/// 3. Up to one [`PlayerActionChoice::MoveToCover`] per reachable cover level better than current.
/// 4. [`PlayerActionChoice::Pass`] (always last).
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

    // ── Ability options ────────────────────────────────────────────────────────
    if let Some(ch) = world.get::<Character>(actor) {
        let level = ch.level;
        let kind = ch.kind.clone();
        let abilities = available_abilities(&kind, level);

        // Collect living enemy data before further borrows.
        let mut q = world.query::<(Entity, &Character, &Health, &Stats, &Position)>();
        let enemies: Vec<(Entity, i32, i32, i32)> = q
            .iter(world)
            .filter(|(_, c, h, _, _)| {
                !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
            })
            .map(|(e, _, _, stats, pos)| (e, stats.defense, pos.x, pos.y))
            .collect();

        let actor_attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);

        for ability in &abilities {
            if ability.ap_cost > ap {
                continue;
            }
            match &ability.kind {
                AbilityKind::Ranged => {
                    for &(target_entity, defense, tx, ty) in &enemies {
                        let dir = Direction::from_attack((actor_pos.x, actor_pos.y), (tx, ty));
                        let cover = world
                            .get_resource::<LevelMap>()
                            .map(|m| m.get_cover(tx, ty, dir))
                            .unwrap_or(CoverLevel::None);
                        let hit_chance = Some(calc_hit_chance(cover));
                        let (damage, cover_opt) =
                            offensive_damage(ability, actor_attack, defense, cover);
                        choices.push(PlayerActionChoice::UseAbility {
                            ability: ability.clone(),
                            target: Some(target_entity),
                            hit_chance,
                            damage,
                            cover: cover_opt,
                        });
                    }
                }
                AbilityKind::Melee => {
                    // Only offer for adjacent targets (Chebyshev ≤ 1, diagonals included).
                    for &(target_entity, defense, tx, ty) in &enemies {
                        let chebyshev = (actor_pos.x - tx).abs().max((actor_pos.y - ty).abs());
                        if chebyshev > 1 {
                            continue;
                        }
                        let dir = Direction::from_attack((actor_pos.x, actor_pos.y), (tx, ty));
                        let cover = world
                            .get_resource::<LevelMap>()
                            .map(|m| m.get_cover(tx, ty, dir))
                            .unwrap_or(CoverLevel::None);
                        let hit_chance = Some(calc_hit_chance(cover));
                        let (damage, cover_opt) =
                            offensive_damage(ability, actor_attack, defense, cover);
                        choices.push(PlayerActionChoice::UseAbility {
                            ability: ability.clone(),
                            target: Some(target_entity),
                            hit_chance,
                            damage,
                            cover: cover_opt,
                        });
                    }
                }
                AbilityKind::RangedAlly => {
                    // Generate one choice per living ally (including the actor themselves).
                    let mut q = world.query::<(Entity, &Character, &Health)>();
                    let allies: Vec<Entity> = q
                        .iter(world)
                        .filter(|(e, c, h)| c.kind.is_player() && h.is_alive() && *e != actor)
                        .map(|(e, _, _)| e)
                        .collect();
                    // Always include self as a target.
                    let mut all_targets = vec![actor];
                    all_targets.extend(allies);
                    for target_entity in all_targets {
                        choices.push(PlayerActionChoice::UseAbility {
                            ability: ability.clone(),
                            target: Some(target_entity),
                            hit_chance: None,
                            damage: None,
                            cover: None,
                        });
                    }
                }
                AbilityKind::RangedAny => {
                    // Generate one choice per living combatant (ally or enemy) at any range.
                    let mut q = world.query::<(Entity, &Health)>();
                    let all_living: Vec<Entity> = q
                        .iter(world)
                        .filter(|(e, h)| h.is_alive() && *e != actor)
                        .map(|(e, _)| e)
                        .collect();
                    for target_entity in all_living {
                        choices.push(PlayerActionChoice::UseAbility {
                            ability: ability.clone(),
                            target: Some(target_entity),
                            hit_chance: None,
                            damage: None,
                            cover: None,
                        });
                    }
                }
                AbilityKind::Utility => {
                    choices.push(PlayerActionChoice::UseAbility {
                        ability: ability.clone(),
                        target: None,
                        hit_chance: None,
                        damage: None,
                        cover: None,
                    });
                }
            }
        }
    }

    // ── Cover move options ─────────────────────────────────────────────────────
    let (cols, rows) = world
        .get_resource::<LevelMap>()
        .map(|m| (m.cols as i32, m.rows as i32))
        .unwrap_or((0, 0));

    if cols > 0 && rows > 0 {
        let speed = world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);
        let range = move_range_per_ap(speed);
        let max_tiles = ap * range;

        let mut q2 = world.query::<(Entity, &Character, &Health, &Position)>();
        let enemy_positions: Vec<(i32, i32)> = q2
            .iter(world)
            .filter(|(_, c, h, _)| {
                !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
            })
            .map(|(_, _, _, pos)| (pos.x, pos.y))
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

            let occupied = crate::turn::occupied_tiles(world, actor);

            let mut candidates: Vec<(CoverLevel, i32, i32, i32)> = Vec::new();
            if let Some(map) = world.get_resource::<LevelMap>() {
                for dy in -(max_tiles)..=max_tiles {
                    for dx in -(max_tiles)..=max_tiles {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let tx = actor_pos.x + dx;
                        let ty = actor_pos.y + dy;
                        if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
                            continue;
                        }
                        if !map.is_passable(tx, ty) || occupied.contains(&(tx, ty)) {
                            continue;
                        }
                        let tile_cover = map.get_cover(tx, ty, attack_dir);
                        if tile_cover <= current_cover {
                            continue;
                        }
                        let path = map.bfs_path((actor_pos.x, actor_pos.y), (tx, ty), &occupied);
                        if path.is_empty() {
                            continue;
                        }
                        let cost = crate::turn::move_ap_cost(path.len() as i32, speed);
                        if cost > ap {
                            continue;
                        }
                        candidates.push((tile_cover, cost, tx, ty));
                    }
                }
            }

            candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut seen: HashSet<u8> = HashSet::new();
            for (tile_cover, cost, tx, ty) in candidates {
                if seen.insert(tile_cover as u8) {
                    choices.push(PlayerActionChoice::MoveToCover {
                        destination: Position::new(tx, ty),
                        cover: tile_cover,
                        ap_cost: cost,
                    });
                }
            }
        }
    }

    choices.push(PlayerActionChoice::Pass);
    choices
}

/// Computes the pre-displayed damage and cover for an offensive ability choice.
/// Returns `(damage, cover_opt)` — damage is `None` for non-damage effects.
fn offensive_damage(
    ability: &Ability,
    actor_attack: i32,
    defense: i32,
    cover: CoverLevel,
) -> (Option<i32>, Option<CoverLevel>) {
    let cover_opt = Some(cover);
    match &ability.effect {
        AbilityEffect::BonusDamage { bonus } => {
            (Some(calc_damage(actor_attack, defense) + bonus), cover_opt)
        }
        AbilityEffect::ArmorPiercing { pierce_fraction } => {
            let eff = (defense as f32 * (1.0 - pierce_fraction)) as i32;
            (Some(calc_damage(actor_attack, eff)), cover_opt)
        }
        AbilityEffect::ArmorPiercingStrike {
            pierce_fraction,
            bonus,
        } => {
            let eff = (defense as f32 * (1.0 - pierce_fraction)) as i32;
            (Some(calc_damage(actor_attack, eff) + bonus), cover_opt)
        }
        AbilityEffect::Displacement { .. } => {
            (Some(calc_damage(actor_attack, defense) + 15), cover_opt)
        }
        AbilityEffect::EntropicRounds => {
            // Ignores defense and cover — raw attack damage.
            (Some(actor_attack.max(1)), None)
        }
        _ => (None, None),
    }
}
