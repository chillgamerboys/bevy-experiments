//! Compact rolled initiative identities; never predicts an unrolled next round.

use super::*;

pub(super) fn value(snapshot: &CombatSnapshot, expanded: bool) -> String {
    let entries = snapshot
        .initiative
        .iter()
        .map(|entry| {
            let identity = snapshot
                .actor(entry.actor)
                .map_or_else(|| "?".to_owned(), |actor| actors::token(snapshot, actor));
            let token = if entry.completed {
                format!("{identity}.")
            } else if snapshot.active_actor == Some(entry.actor) {
                format!("[{identity}]")
            } else {
                identity
            };
            if expanded {
                format!("{token} {}+{}={}", entry.speed, entry.roll, entry.total)
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(if expanded { "  /  " } else { "  " });
    if expanded {
        format!("This round: Speed + d8 | [acting] .completed\n{entries}")
    } else {
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labyrinth_rules::{Combat, DEFAULT_HERO_ROSTER};

    #[test]
    fn compact_order_keeps_all_rolled_actors_and_expands_only_on_request() {
        let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let first = snapshot.initiative.first_mut().expect("first");
        first.completed = true;
        let first_actor = first.actor;
        let second = snapshot.initiative.get_mut(1).expect("second");
        second.completed = false;
        let second_actor = second.actor;
        let roll_text = format!("{}+{}={}", second.speed, second.roll, second.total);
        snapshot.active_actor = Some(second_actor);
        let compact = value(&snapshot, false);
        assert_eq!(compact.split_whitespace().count(), PARTY_SIZE * 2);
        assert!(!compact.contains("Speed"));
        assert!(!compact.contains('+'));
        let completed = snapshot.actor(first_actor).expect("actor");
        let active = snapshot.actor(second_actor).expect("actor");
        assert!(compact.contains(&format!("{}.", actors::token(&snapshot, completed))));
        assert!(compact.contains(&format!("[{}]", actors::token(&snapshot, active))));
        let expanded = value(&snapshot, true);
        assert!(expanded.contains("Speed + d8"));
        assert!(expanded.contains(&roll_text));
    }
}
