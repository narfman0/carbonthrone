/// Common interface for anything that participates in combat.
pub trait Combatant {
    fn name(&self) -> &str;
    fn current_hp(&self) -> i32;
    fn max_hp(&self) -> i32;
    fn attack(&self) -> i32;
    fn defense(&self) -> i32;
    fn speed(&self) -> i32;
    fn take_damage(&mut self, amount: i32);
    fn is_alive(&self) -> bool;
}
