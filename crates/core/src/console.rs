use bevy::prelude::*;

use crate::{
    character::{Aggression, Character},
    combat::BattleOutcome,
    game::{GamePhase, GameSession},
    health::Health,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConsoleCommand {
    Help,
    DefeatEnemies,
    SetLoop(u32),
    FlagList,
    FlagSet(String),
    FlagClear(String),
    SetCompanion(Option<String>),
    Unknown(String),
}

pub fn parse_command(input: &str) -> ConsoleCommand {
    let trimmed = input.trim();

    if let Some(rest) = trimmed.strip_prefix("loop ") {
        return match rest.trim().parse::<u32>() {
            Ok(n) => ConsoleCommand::SetLoop(n),
            Err(_) => ConsoleCommand::Unknown(trimmed.to_string()),
        };
    }
    if let Some(rest) = trimmed.strip_prefix("flag set ") {
        let name = rest.trim().to_string();
        return if name.is_empty() {
            ConsoleCommand::Unknown(trimmed.to_string())
        } else {
            ConsoleCommand::FlagSet(name)
        };
    }
    if let Some(rest) = trimmed.strip_prefix("flag clear ") {
        let name = rest.trim().to_string();
        return if name.is_empty() {
            ConsoleCommand::Unknown(trimmed.to_string())
        } else {
            ConsoleCommand::FlagClear(name)
        };
    }
    if let Some(rest) = trimmed.strip_prefix("companion ") {
        let name = rest.trim();
        return match name {
            "none" => ConsoleCommand::SetCompanion(None),
            "orin" | "doss" | "kaleo" => ConsoleCommand::SetCompanion(Some(name.to_string())),
            _ => ConsoleCommand::Unknown(trimmed.to_string()),
        };
    }
    match trimmed {
        "help" => ConsoleCommand::Help,
        "kill" => ConsoleCommand::DefeatEnemies,
        "flag list" => ConsoleCommand::FlagList,
        other => ConsoleCommand::Unknown(other.to_string()),
    }
}

pub fn defeat_all_enemies(world: &mut World) {
    let entities: Vec<Entity> = {
        let mut q = world.query::<(Entity, &Character)>();
        q.iter(world)
            .filter(|(_, c)| !c.kind.is_player() && c.aggression != Aggression::Friendly)
            .map(|(e, _)| e)
            .collect()
    };
    for entity in entities {
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.current = 0;
        }
    }
}

pub fn execute_command(cmd: ConsoleCommand, session: &mut GameSession) -> String {
    match cmd {
        ConsoleCommand::Help => {
            "Commands: help, kill, loop <1-5>, flag list, flag set <name>, flag clear <name>, companion <orin|doss|kaleo|none>, worldbuilding, worldbuilding exit".to_string()
        }
        ConsoleCommand::DefeatEnemies => {
            defeat_all_enemies(&mut session.world);
            "All enemies defeated.".to_string()
        }
        ConsoleCommand::SetLoop(n) => {
            let clamped = n.clamp(1, 5);
            session.goto_loop(clamped);
            format!("Jumped to loop {clamped}.")
        }
        ConsoleCommand::FlagList => {
            let flags = exploration_flags(session);
            if flags.is_empty() {
                "No flags set.".to_string()
            } else {
                flags.join(" ")
            }
        }
        ConsoleCommand::FlagSet(name) => {
            let GamePhase::Exploration(e) = &mut session.phase else {
                return "Not in exploration.".to_string();
            };
            e.flags.set_flag(&name);
            format!("Flag set: {name}")
        }
        ConsoleCommand::FlagClear(name) => {
            let GamePhase::Exploration(e) = &mut session.phase else {
                return "Not in exploration.".to_string();
            };
            if e.flags.clear_flag(&name) {
                format!("Flag cleared: {name}")
            } else {
                format!("Flag not set: {name}")
            }
        }
        ConsoleCommand::SetCompanion(companion) => {
            let GamePhase::Exploration(e) = &mut session.phase else {
                return "Not in exploration.".to_string();
            };
            e.active_companion = companion.clone();
            match companion {
                Some(name) => format!("Companion set to: {name}"),
                None => "Companion cleared.".to_string(),
            }
        }
        ConsoleCommand::Unknown(s) => {
            format!("Unknown command: {s}, use 'help' for a list of commands.")
        }
    }
}

pub fn check_battle_outcome(session: &mut GameSession) -> Option<BattleOutcome> {
    crate::combat::check_outcome(&mut session.world)
}

fn exploration_flags(session: &mut GameSession) -> Vec<String> {
    match &session.phase {
        GamePhase::Exploration(e) | GamePhase::Battle(e) => e.flags.export_flags(),
        _ => vec![],
    }
}
