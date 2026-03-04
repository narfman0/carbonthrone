use carbonthrone::character::{Character, CharacterClass};
use carbonthrone::party::{Party, MAX_PARTY_SIZE};

fn make_char(name: &str) -> Character {
    Character::new(name, CharacterClass::Warrior)
}

#[test]
fn can_add_up_to_five_members() {
    let mut party = Party::new();
    for i in 0..5 {
        assert!(party.add_member(make_char(&format!("Hero{i}"))).is_ok());
    }
    assert_eq!(party.size(), 5);
}

#[test]
fn cannot_exceed_max_party_size() {
    let mut party = Party::new();
    for i in 0..MAX_PARTY_SIZE {
        party.add_member(make_char(&format!("Hero{i}"))).unwrap();
    }
    assert!(party.add_member(make_char("Extra")).is_err());
}

#[test]
fn remove_member_works() {
    let mut party = Party::new();
    party.add_member(make_char("Alice")).unwrap();
    party.add_member(make_char("Bob")).unwrap();
    let removed = party.remove_member(0).unwrap();
    assert_eq!(removed.name, "Alice");
    assert_eq!(party.size(), 1);
}

#[test]
fn is_wiped_when_all_dead() {
    let mut party = Party::new();
    party.add_member(make_char("Alice")).unwrap();
    party.members_mut()[0].take_damage(9999);
    assert!(party.is_wiped());
}

#[test]
fn not_wiped_when_one_alive() {
    let mut party = Party::new();
    party.add_member(make_char("Alice")).unwrap();
    party.add_member(make_char("Bob")).unwrap();
    party.members_mut()[0].take_damage(9999);
    assert!(!party.is_wiped());
}
