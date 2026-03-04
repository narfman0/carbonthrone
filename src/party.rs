use bevy::prelude::{Entity, Resource, World};
use crate::health::Health;

pub const MAX_PARTY_SIZE: usize = 5;

/// Global resource holding the player's party as a list of entity IDs.
/// Each entity is expected to carry Character, Health, Stats, ActionPoints, and Experience.
#[derive(Debug, Resource)]
pub struct Party {
    members: Vec<Entity>,
}

impl Party {
    pub fn new() -> Self {
        Self { members: Vec::new() }
    }

    /// Returns `Err` if the party is already full.
    pub fn add_member(&mut self, entity: Entity) -> Result<(), &'static str> {
        if self.members.len() >= MAX_PARTY_SIZE {
            return Err("Party is full (max 5 members)");
        }
        self.members.push(entity);
        Ok(())
    }

    pub fn remove_member(&mut self, index: usize) -> Option<Entity> {
        if index < self.members.len() {
            Some(self.members.remove(index))
        } else {
            None
        }
    }

    pub fn members(&self) -> &[Entity] {
        &self.members
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTY_SIZE
    }

    /// Returns true if every member's Health is at zero (or the entity has no Health).
    pub fn is_wiped(&self, world: &World) -> bool {
        self.members.iter().all(|&e| {
            world.get::<Health>(e).is_none_or(|h| !h.is_alive())
        })
    }
}

impl Default for Party {
    fn default() -> Self {
        Self::new()
    }
}
