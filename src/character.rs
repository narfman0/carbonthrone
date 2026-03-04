use bevy::prelude::Component;
use crate::combatant::Combatant;
use crate::stats::Stats;

#[derive(Debug, Clone, PartialEq, Component)]
pub enum CharacterClass {
    Warrior,
    Mage,
    Rogue,
    Cleric,
    Ranger,
}

#[derive(Debug, Clone, Component)]
pub struct Character {
    pub name: String,
    pub class: CharacterClass,
    pub level: u32,
    pub stats: Stats,
    pub current_hp: i32,
    pub experience: u32,
}

impl Character {
    pub fn new(name: impl Into<String>, class: CharacterClass) -> Self {
        let stats = Stats::for_class(&class);
        let current_hp = stats.max_hp;
        Self {
            name: name.into(),
            class,
            level: 1,
            stats,
            current_hp,
            experience: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.current_hp = (self.current_hp - amount).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current_hp = (self.current_hp + amount).min(self.stats.max_hp);
    }

    pub fn gain_experience(&mut self, amount: u32) {
        self.experience += amount;
        while self.experience >= self.xp_to_next_level() {
            self.experience -= self.xp_to_next_level();
            self.level_up();
        }
    }

    fn xp_to_next_level(&self) -> u32 {
        100 * self.level
    }

    fn level_up(&mut self) {
        self.level += 1;
        self.stats.level_up(&self.class);
        self.current_hp = self.stats.max_hp;
    }
}

impl Combatant for Character {
    fn name(&self) -> &str         { &self.name }
    fn current_hp(&self) -> i32    { self.current_hp }
    fn max_hp(&self) -> i32        { self.stats.max_hp }
    fn attack(&self) -> i32        { self.stats.attack }
    fn defense(&self) -> i32       { self.stats.defense }
    fn speed(&self) -> i32         { self.stats.speed }
    fn magic(&self) -> i32         { self.stats.magic }
    fn take_damage(&mut self, amount: i32) { Character::take_damage(self, amount); }
    fn is_alive(&self) -> bool     { Character::is_alive(self) }
}
