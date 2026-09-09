//! This round's rolled initiative; never predicts an unrolled next round.

use super::*;

pub(super) fn value(snapshot: &CombatSnapshot, ui: &UiState, viewport: UiViewportClass) -> String {
    if viewport == UiViewportClass::Compact && !ui.show_timeline {
        let next = snapshot
            .initiative
            .iter()
            .filter(|entry| !entry.completed)
            .take(2)
            .map(|entry| {
                format!(
                    "{} {}+{}={}",
                    snapshot.actor(entry.actor).map_or("?", ActorSnapshot::name),
                    entry.speed,
                    entry.roll,
                    entry.total
                )
            })
            .collect::<Vec<_>>()
            .join(" | then ");
        return format!("INITIATIVE | {next}");
    }
    let entries = snapshot
        .initiative
        .iter()
        .map(|entry| {
            let name = snapshot.actor(entry.actor).map_or("?", ActorSnapshot::name);
            let mark = if entry.completed {
                "done"
            } else if snapshot.active_actor == Some(entry.actor) {
                "NOW"
            } else {
                "next"
            };
            format!(
                "{name} {}+{}={} [{mark}]",
                entry.speed, entry.roll, entry.total
            )
        })
        .collect::<Vec<_>>()
        .join("  /  ");
    format!("THIS ROUND | Speed + d8\n{entries}")
}
