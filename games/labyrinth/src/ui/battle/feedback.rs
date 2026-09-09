//! Bounded outcome feedback; reconnect snapshots are a baseline, not an animation replay.

use super::*;

pub(super) fn update(nodes: &mut BattleNodes, view: &LabyrinthView, time: f64) {
    let highest = view.events.last().map_or(0, |event| event.id);
    if nodes.last_event.is_none() || view.paused || !view.admitted || nodes.was_paused {
        // First snapshots and recovery are a baseline, not an animation replay.
        nodes.last_event = Some(highest);
        nodes.feedback.clear();
    } else {
        let previous = nodes.last_event.unwrap_or(0);
        let fresh = view
            .events
            .iter()
            .rev()
            .filter(|event| event.id > previous)
            .take(32)
            .collect::<Vec<_>>();
        for event in fresh.into_iter().rev() {
            let feedback = match event.event.kind {
                CombatEventKind::Damage {
                    target,
                    amount,
                    kind,
                    ..
                } => Some((
                    target,
                    format!(
                        "-{amount} {}",
                        if kind == DamageKind::Bleed {
                            "BLEED"
                        } else {
                            "HP"
                        }
                    ),
                )),
                CombatEventKind::Healed { target, amount, .. } => {
                    Some((target, format!("+{amount} HP")))
                }
                CombatEventKind::Rescued { actor, hp, .. } => {
                    Some((actor, format!("RESCUED +{hp}")))
                }
                _ => None,
            };
            if let Some((actor, message)) = feedback {
                nodes.feedback.insert(actor, (message, time + 1.2));
            }
        }
        nodes.last_event = Some(highest.max(previous));
    }
    nodes.was_paused = view.paused || !view.admitted;
    nodes.feedback.retain(|_, (_, until)| *until > time);
}
