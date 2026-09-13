//! Character decisions are projected from the real build resolver, never a second rules model.
use super::*;
use labyrinth_rules::{
    build::{CharacterBuild, GrantKind, ResolvedAbility, ResolvedBuild},
    catalog::{AbilityDefinition, TargetPattern},
    TargetRule, Team,
};

pub(super) fn rank_span(view: &LabyrinthView, editor: &ActorEditor) -> (Team, u8, u8) {
    let Some(scenario) = &view.scenario else {
        return (Team::Heroes, 1, 1);
    };
    let (team, roster) = if scenario.heroes.iter().any(|a| a.id == editor.id) {
        (Team::Heroes, &scenario.heroes)
    } else {
        (Team::Enemies, &scenario.enemies)
    };
    let start = 1 + roster
        .iter()
        .take_while(|a| a.id != editor.id)
        .map(|a| a.actor.footprint)
        .sum::<u8>();
    let width = editor
        .footprint
        .parse::<u8>()
        .unwrap_or(editor.draft.actor.footprint)
        .clamp(1, 6);
    (team, start, start.saturating_add(width - 1).min(6))
}
pub(super) fn ranks(mask: u8) -> String {
    (1..=6)
        .filter(|rank| mask & (1 << (rank - 1)) != 0)
        .map(|rank| rank.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
pub(super) fn move_summary(ability: &AbilityDefinition) -> String {
    format!(
        "{} · acting ranks {}",
        crate::presentation::effects_description(&ability.effects),
        ranks(ability.source_ranks)
    )
}
pub(super) fn move_facts(
    ability: &ResolvedAbility,
    span: (u8, u8),
    catalog: &ContentCatalog,
) -> Vec<String> {
    let def = &ability.definition;
    let target = match def.target_rule {
        TargetRule::EnemyStanding => "standing enemies",
        TargetRule::AllyStanding => "standing allies, including self",
        TargetRule::SelfStanding => "self",
        TargetRule::OtherAlly => "another ally, including downed allies",
        TargetRule::AllyDowned => "a downed hero ally",
    };
    let position = if (span.0..=span.1).any(|rank| def.allows_source_rank(rank)) {
        "Your current position allows this move.".to_owned()
    } else {
        format!("Unavailable at current rank {}. The move remains in your build; reposition to an allowed acting rank.", span.0)
    };
    let mut facts = vec![
        crate::presentation::effects_description(&def.effects),
        format!(
            "Acting ranks: {}\nTarget ranks: {} · {target}",
            ranks(def.source_ranks),
            ranks(def.target_ranks)
        ),
        position,
        if def.target_pattern == TargetPattern::FrontPair {
            "Targets both front ranks 1–2. Each distinct occupant is hit once, including a creature occupying both ranks.".into()
        } else {
            "One selected target.".into()
        },
        def.max_uses.map_or_else(
            || "Unlimited uses per encounter.".into(),
            |uses| format!("{uses} uses per encounter."),
        ),
    ];
    let sources = ability
        .grants
        .iter()
        .map(|grant| match grant.kind {
            GrantKind::Weapon => format!(
                "weapon: {}",
                catalog
                    .weapon(&grant.definition)
                    .map_or(grant.definition.as_str(), |v| v.name.as_str())
            ),
            GrantKind::Learned => format!(
                "learned: {} ({})",
                catalog
                    .learned_skill(&grant.definition)
                    .map_or(grant.definition.as_str(), |v| v.name.as_str()),
                grant.provenance
            ),
            GrantKind::Innate => format!("innate ({})", grant.provenance),
        })
        .collect::<Vec<_>>()
        .join("; ");
    facts.push(if sources.is_empty() {
        "Not granted by this build. Previewing the base definition.".into()
    } else {
        format!("Granted by {sources}.")
    });
    for upgrade in &ability.upgrades {
        facts.push(format!(
            "Upgraded by {} ({}) · {}",
            catalog
                .learned_skill(&upgrade.source.definition)
                .map_or(upgrade.source.definition.as_str(), |v| v.name.as_str()),
            upgrade.source.provenance,
            upgrade
                .upgrade
                .operations
                .iter()
                .map(operation)
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    facts
}
fn operation(value: &labyrinth_rules::catalog::UpgradeOperation) -> String {
    use labyrinth_rules::catalog::UpgradeOperation::*;
    match value {
        AddDamage { amount, .. } => format!("+{amount} direct damage"),
        AppendEffect(effect) => {
            crate::presentation::effects_description(std::slice::from_ref(effect))
        }
        ExtendSourceRanks(mask) => format!("adds acting ranks {}", ranks(*mask)),
        ExtendTargetRanks(mask) => format!("adds target ranks {}", ranks(*mask)),
    }
}
pub(super) fn proposed(
    editor: &ActorEditor,
    selection: &Selection,
    catalog: &ContentCatalog,
) -> CharacterBuild {
    let mut build = editor.draft.actor.build.clone();
    match selection {
        Selection::Weapon(id) => build.weapon.clone_from(id),
        Selection::Innate(id) => {
            if build.innate.iter().any(|g| &g.ability == id) {
                build.innate.retain(|g| &g.ability != id);
            } else {
                build.innate.push(InnateGrant {
                    ability: id.clone(),
                    provenance: ContentId::new("innate").expect("constant"),
                });
            }
        }
        Selection::Learned(id) => {
            if build.learned_skills.contains(id) {
                build.learned_skills.retain(|v| v != id);
            } else {
                build.learned_skills.push(id.clone());
            }
        }
        Selection::Preset(id) => {
            if let Some(preset) = catalog.actor_preset(id) {
                build = preset.build.clone();
            }
        }
        Selection::Move(_) => {}
    }
    build
}
pub(super) fn changes(before: &ResolvedBuild, after: &ResolvedBuild) -> Vec<String> {
    let mut lines = Vec::new();
    for old in &before.abilities {
        match after
            .abilities
            .iter()
            .find(|new| new.definition.id == old.definition.id)
        {
            None => lines.push(format!("Removed · {}", old.definition.name)),
            Some(new) if new.definition != old.definition => {
                let mut changed = Vec::new();
                if old.definition.effects != new.definition.effects {
                    changed.push(format!(
                        "{} → {}",
                        crate::presentation::effects_description(&old.definition.effects),
                        crate::presentation::effects_description(&new.definition.effects)
                    ));
                }
                if old.definition.source_ranks != new.definition.source_ranks {
                    changed.push(format!(
                        "acting ranks {} → {}",
                        ranks(old.definition.source_ranks),
                        ranks(new.definition.source_ranks)
                    ));
                }
                if old.definition.target_ranks != new.definition.target_ranks {
                    changed.push(format!(
                        "target ranks {} → {}",
                        ranks(old.definition.target_ranks),
                        ranks(new.definition.target_ranks)
                    ));
                }
                if old.definition.max_uses != new.definition.max_uses {
                    changed.push(format!(
                        "uses {:?} → {:?}",
                        old.definition.max_uses, new.definition.max_uses
                    ));
                }
                if old.definition.target_pattern != new.definition.target_pattern {
                    changed.push(format!(
                        "target pattern {:?} → {:?}",
                        old.definition.target_pattern, new.definition.target_pattern
                    ));
                }
                lines.push(format!(
                    "Changed · {}: {}",
                    new.definition.name,
                    changed.join("; ")
                ));
            }
            Some(new) if new.grants != old.grants || new.upgrades != old.upgrades => {
                lines.push(format!(
                    "Retained · {} · grant sources {} → {}. Other grants keep this move available.",
                    new.definition.name,
                    old.grants.len(),
                    new.grants.len()
                ))
            }
            _ => {}
        }
    }
    for new in &after.abilities {
        if !before
            .abilities
            .iter()
            .any(|old| old.definition.id == new.definition.id)
        {
            lines.push(format!(
                "Added · {} · {}",
                new.definition.name,
                move_summary(&new.definition)
            ));
        }
    }
    if lines.is_empty() {
        lines.push("No change to active moves.".into());
    }
    lines
}
pub(super) struct Inspection {
    pub title: String,
    pub description: String,
    pub facts: Vec<String>,
    pub moves: Vec<ResolvedAbility>,
    pub changes: Vec<String>,
    pub apply: Option<(String, bool)>,
}
pub(super) fn inspection(editor: &ActorEditor, catalog: &ContentCatalog) -> Inspection {
    let mut result=Inspection {title:"Battle parameters".into(),description:"Set health, initiative speed, occupied ranks and starting conditions for this encounter.".into(),facts:vec![],moves:vec![],changes:vec![],apply:None};
    let Some(selected) = editor.selection() else {
        return result;
    };
    let current = catalog.resolve_build(&editor.draft.actor.build);
    let candidate = proposed(editor, selected, catalog);
    let next = catalog.resolve_build(&candidate);
    let mut move_ids = Vec::new();
    match selected {
        Selection::Weapon(id) => {
            if let Some(weapon) = id.as_ref().and_then(|id| catalog.weapon(id)) {
                result.title = weapon.name.clone();
                result.description = weapon.description.clone();
                move_ids = weapon.grants.clone();
                result.facts.push(
                    match weapon.handedness {
                        labyrinth_rules::catalog::Handedness::One => {
                            "One-handed weapon · one optional equipment choice."
                        }
                        labyrinth_rules::catalog::Handedness::Two => {
                            "Two-handed weapon · one optional equipment choice."
                        }
                    }
                    .into(),
                );
            } else {
                result.title = "Unarmed".into();
                result.description =
                    "Remove the equipped weapon. Innate and learned grants remain.".into();
            }
            let equipped = editor.draft.actor.build.weapon == *id;
            result.facts.push(
                if equipped {
                    "Equipped in your draft."
                } else {
                    "Inspecting an alternative. Your draft has not changed."
                }
                .into(),
            );
            result.apply = Some((
                if equipped {
                    "Equipped".into()
                } else {
                    format!("Equip {}", result.title)
                },
                equipped,
            ));
        }
        Selection::Innate(id) => {
            if let Some(ability) = catalog.ability(id) {
                result.title = ability.name.clone();
                result.description = ability.description.clone();
                move_ids.push(id.clone());
                let granted = editor
                    .draft
                    .actor
                    .build
                    .innate
                    .iter()
                    .any(|g| g.ability == *id);
                result
                    .facts
                    .push("Innate grants do not require a weapon or learned technique.".into());
                result.apply = Some((
                    if granted {
                        "Remove innate grant"
                    } else {
                        "Add innate grant"
                    }
                    .into(),
                    false,
                ));
            }
        }
        Selection::Learned(id) => {
            if let Some(skill) = catalog.learned_skill(id) {
                result.title = skill.name.clone();
                result.description = skill.description.clone();
                move_ids = skill.grants.clone();
                result.facts.push(format!(
                    "Discipline: {}. Available to any character with the required move.",
                    skill.provenance
                ));
                for upgrade in &skill.upgrades {
                    move_ids.push(upgrade.ability.clone());
                    result.facts.push(format!(
                        "Requires {} from any grant source. {}",
                        catalog
                            .ability(&upgrade.ability)
                            .map_or(upgrade.ability.as_str(), |a| a.name.as_str()),
                        upgrade
                            .operations
                            .iter()
                            .map(operation)
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
                if skill.upgrades.is_empty() {
                    result.facts.push("No prerequisite move required.".into());
                }
                result.apply = Some((
                    if editor.draft.actor.build.learned_skills.contains(id) {
                        "Forget technique"
                    } else {
                        "Learn technique"
                    }
                    .into(),
                    false,
                ));
            }
        }
        Selection::Preset(id) => {
            if let Some(preset) = catalog.actor_preset(id) {
                result.title = preset.name.clone();
                result.description="Use this preset's appearance, equipment, innate moves and starting values in your draft.".into();
                result.facts.push(format!("Maximum HP {} → {} · speed {} → {} · formation spaces {} → {}. Starting HP resets to full.",editor.max_hp,preset.max_hp,editor.speed,preset.base_speed,editor.footprint,preset.footprint));
                move_ids = catalog.resolve_build(&preset.build).map_or_else(
                    |_| vec![],
                    |build| {
                        build
                            .abilities
                            .iter()
                            .map(|a| a.definition.id.clone())
                            .collect()
                    },
                );
                result.apply = Some(("Use preset in draft".into(), false));
            }
        }
        Selection::Move(id) => {
            move_ids.push(id.clone());
            if let Some(ability) = catalog.ability(id) {
                result.title = ability.name.clone();
                result.description="Effective move in your current draft, including every grant and learned upgrade.".into();
            }
        }
    }
    let resolved = next.as_ref().ok().or_else(|| current.as_ref().ok());
    if let Some(build) = resolved {
        for id in move_ids {
            if let Some(ability) = build.abilities.iter().find(|a| a.definition.id == id) {
                if !result.moves.iter().any(|a| a.definition.id == id) {
                    result.moves.push(ability.clone());
                }
            } else if let Some(def) = catalog.ability(&id) {
                result.moves.push(ResolvedAbility {
                    definition: def.clone(),
                    grants: vec![],
                    upgrades: vec![],
                });
            }
        }
    }
    if !matches!(selected, Selection::Move(_)) {
        match (&current, &next) {
            (Ok(before), Ok(after)) => result.changes = changes(before, after),
            (_, Err(error)) => {
                result
                    .changes
                    .push(format!("Cannot apply this choice: {error}"));
                if let Some((_, disabled)) = &mut result.apply {
                    *disabled = true;
                }
            }
            (Err(error), _) => result
                .changes
                .push(format!("Current draft needs correction: {error}")),
        }
    }
    result
}
