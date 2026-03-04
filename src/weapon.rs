use bevy::prelude::Component;

#[derive(Debug, Clone, PartialEq, Component)]
pub enum WeaponKind {
    Melee,
    Ranged,
}

/// A weapon carried by a combatant. Attach as a Bevy Component alongside a Character or Enemy.
#[derive(Debug, Clone, Component)]
pub struct Weapon {
    pub name: String,
    pub kind: WeaponKind,
    /// Flat damage this weapon deals (added to attacker's base attack in calc_damage).
    pub damage: i32,
    /// Action points required to switch to this weapon.
    pub switch_cost: u32,
}

impl Weapon {
    pub fn melee(name: impl Into<String>, damage: i32, switch_cost: u32) -> Self {
        Self { name: name.into(), kind: WeaponKind::Melee, damage, switch_cost }
    }

    pub fn ranged(name: impl Into<String>, damage: i32, switch_cost: u32) -> Self {
        Self { name: name.into(), kind: WeaponKind::Ranged, damage, switch_cost }
    }
}
