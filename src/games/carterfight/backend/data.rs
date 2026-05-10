use super::moves::{MoveDef, MoveEffect, MoveId};
use super::state::Character;

/// Look up a move by id. `None` for unknown ids — caller decides what to do
/// (panic in dev, silently skip, etc.).
pub fn move_def(id: MoveId) -> Option<MoveDef> {
    moves().into_iter().find(|m| m.id == id)
}

/// Build a fresh `Character` from a template name. Returns `None` if the
/// template doesn't exist.
pub fn character_template(name: &str) -> Option<Character> {
    characters().into_iter().find(|c| c.name == name)
}

fn moves() -> Vec<MoveDef> {
    vec![
        MoveDef {
            id: "jab",
            name: "Jab",
            description: "A quick, low-damage strike.",
            effect: MoveEffect::Damage { amount: 8 },
        },
        MoveDef {
            id: "haymaker",
            name: "Haymaker",
            description: "A heavy swing — slow but it hurts.",
            effect: MoveEffect::Damage { amount: 20 },
        },
        MoveDef {
            id: "headbutt",
            name: "Headbutt",
            description: "A blunt skull-first attack.",
            effect: MoveEffect::Damage { amount: 14 },
        },
    ]
}

fn characters() -> Vec<Character> {
    vec![
        Character {
            name: "Carter".to_string(),
            max_hp: 60,
            current_hp: 60,
            moves: vec!["jab", "haymaker", "headbutt"],
            abilities: vec![],
        },
        Character {
            name: "Rival".to_string(),
            max_hp: 60,
            current_hp: 60,
            moves: vec!["jab", "haymaker"],
            abilities: vec![],
        },
    ]
}
