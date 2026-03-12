use bevy::prelude::*;

use crate::{
    character::{Aggression, Character},
    combat::BattleOutcome,
    game::GameSession,
    health::Health,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConsoleCommand {
    Help,
    DefeatEnemies,
    Unknown(String),
}

pub fn parse_command(input: &str) -> ConsoleCommand {
    match input.trim() {
        "help" => ConsoleCommand::Help,
        "kill" => ConsoleCommand::DefeatEnemies,
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
        ConsoleCommand::Help => "Commands: help, kill".to_string(),
        ConsoleCommand::DefeatEnemies => {
            defeat_all_enemies(&mut session.world);
            "All enemies defeated.".to_string()
        }
        ConsoleCommand::Unknown(s) => format!("Unknown command: {s}, use 'help' for a list of commands."),
    }
}

pub fn check_battle_outcome(session: &mut GameSession) -> Option<BattleOutcome> {
    crate::combat::check_outcome(&mut session.world)
}
