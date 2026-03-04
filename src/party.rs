use bevy::prelude::Resource;
use crate::character::Character;

pub const MAX_PARTY_SIZE: usize = 5;

#[derive(Debug, Resource)]
pub struct Party {
    members: Vec<Character>,
}

impl Party {
    pub fn new() -> Self {
        Self { members: Vec::new() }
    }

    /// Returns `Err` if the party is already full.
    pub fn add_member(&mut self, character: Character) -> Result<(), &'static str> {
        if self.members.len() >= MAX_PARTY_SIZE {
            return Err("Party is full (max 5 members)");
        }
        self.members.push(character);
        Ok(())
    }

    pub fn remove_member(&mut self, index: usize) -> Option<Character> {
        if index < self.members.len() {
            Some(self.members.remove(index))
        } else {
            None
        }
    }

    pub fn members(&self) -> &[Character] {
        &self.members
    }

    pub fn members_mut(&mut self) -> &mut [Character] {
        &mut self.members
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTY_SIZE
    }

    /// All members are dead.
    pub fn is_wiped(&self) -> bool {
        self.members.iter().all(|m| !m.is_alive())
    }

    pub fn living_members(&self) -> impl Iterator<Item = &Character> {
        self.members.iter().filter(|m| m.is_alive())
    }
}

impl Default for Party {
    fn default() -> Self {
        Self::new()
    }
}
