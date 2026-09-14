//! Character decisions are projected from the real build resolver, never a second rules model.
use super::*;
use labyrinth_rules::{
    build::{CharacterBuild, GrantKind, ResolvedBuild, ResolvedSkill},
    catalog::{ContentError, SkillDefinition, TargetPattern},
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
    let start = view
        .formation
        .as_ref()
        .and_then(|f| f.rank(editor.id))
        .unwrap_or_else(|| {
            1 + roster
                .iter()
                .take_while(|a| a.id != editor.id)
                .map(|a| a.actor.footprint)
                .sum::<u8>()
        });
    let width = editor
        .footprint
        .parse::<u8>()
        .unwrap_or(editor.draft.actor.footprint)
        .clamp(1, 6);
    (team, start, start.saturating_add(width - 1).min(6))
}
pub(in crate::ui) fn ranks(mask: u8) -> String {
    (1..=6)
        .filter(|rank| mask & (1 << (rank - 1)) != 0)
        .map(|rank| rank.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
pub(super) fn move_summary(skill: &SkillDefinition) -> String {
    format!(
        "{} · acting ranks {}",
        crate::presentation::effects_description(&skill.effects),
        ranks(skill.source_ranks)
    )
}
pub(in crate::ui) fn move_facts(
    skill: &ResolvedSkill,
    span: (u8, u8),
    catalog: &ContentCatalog,
) -> Vec<String> {
    let def = &skill.definition;
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
    facts.extend(requirements(&def.requirements, catalog));
    for effect in &def.effects {
        if let labyrinth_rules::Effect::ApplyStatus(kind) = effect {
            let explanation = labyrinth_rules::status_definition(*kind)
                .description
                .to_owned();
            if !facts.contains(&explanation) {
                facts.push(explanation);
            }
        }
    }
    let sources = skill
        .grants
        .iter()
        .map(|grant| match grant.kind {
            GrantKind::Equipment => format!(
                "weapon: {}",
                catalog
                    .weapon(&grant.definition)
                    .map_or(grant.definition.as_str(), |v| v.name.as_str())
            ),
            GrantKind::Character => format!("character ({})", grant.provenance),
        })
        .collect::<Vec<_>>()
        .join("; ");
    facts.push(if sources.is_empty() {
        "Not granted by this build. Previewing the base definition.".into()
    } else {
        format!("Granted by {sources}.")
    });
    for upgrade in &skill.upgrades {
        facts.push(format!(
            "Upgraded by {} ({}) · {}",
            catalog
                .ability(&upgrade.ability)
                .map_or(upgrade.ability.as_str(), |v| v.name.as_str()),
            catalog
                .ability(&upgrade.ability)
                .map_or("", |a| a.provenance.as_str()),
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
pub(super) fn requirements(
    values: &[labyrinth_rules::catalog::EquipmentRequirement],
    catalog: &ContentCatalog,
) -> Vec<String> {
    use labyrinth_rules::catalog::EquipmentRequirement;
    if values.is_empty() {
        return vec!["No equipment prerequisite.".into()];
    }
    values
        .iter()
        .map(|requirement| match requirement {
            EquipmentRequirement::Kind(kind) => format!("Requires {kind} equipment."),
            EquipmentRequirement::Item(item) => format!(
                "Requires {}.",
                catalog
                    .weapon(item)
                    .map_or(item.as_str(), |item| item.name.as_str())
            ),
        })
        .collect()
}
pub(super) fn proposed(
    editor: &ActorEditor,
    selection: &Selection,
    _catalog: &ContentCatalog,
) -> CharacterBuild {
    let mut build = editor.draft.actor.build.clone();
    match selection {
        Selection::Weapon(id) => {
            build.weapon = if id.is_some() && build.weapon == *id {
                None
            } else {
                id.clone()
            };
        }
        Selection::Skill(id) => {
            if build.skills.iter().any(|g| &g.skill == id) {
                build.skills.retain(|g| &g.skill != id);
            } else {
                build.skills.push(SkillGrant {
                    skill: id.clone(),
                    provenance: ContentId::new("skills").expect("constant"),
                });
            }
        }
        Selection::Ability(id) => {
            if build.abilities.contains(id) {
                build.abilities.retain(|v| v != id);
            } else {
                build.abilities.push(id.clone());
            }
        }
        Selection::Move(_) | Selection::EquipmentSkill(_) | Selection::EquipmentAbility(_) => {}
    }
    build
}
pub(super) fn changes(before: &ResolvedBuild, after: &ResolvedBuild) -> Vec<String> {
    let mut lines = Vec::new();
    for old in &before.moveset.skills {
        match after
            .moveset
            .skills
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
    for new in &after.moveset.skills {
        if !before
            .moveset
            .skills
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
    for ability in &after.abilities {
        match before
            .abilities
            .iter()
            .find(|a| a.definition.id == ability.definition.id)
        {
            None => lines.push(format!(
                "Added Ability · {} · {}",
                ability.definition.name,
                ability.inactive_reason.as_deref().unwrap_or("active")
            )),
            Some(old) if old.inactive_reason != ability.inactive_reason => lines.push(format!(
                "Ability · {} · {}",
                ability.definition.name,
                ability.inactive_reason.as_deref().unwrap_or("active")
            )),
            _ => {}
        }
    }
    for ability in &before.abilities {
        if !after
            .abilities
            .iter()
            .any(|a| a.definition.id == ability.definition.id)
        {
            lines.push(format!("Removed Ability · {}", ability.definition.name));
        }
    }
    if lines.is_empty() {
        lines.push("No change to Moveset or Abilities.".into());
    }
    lines
}
pub(super) struct Inspection {
    pub title: String,
    pub description: String,
    pub facts: Vec<String>,
    pub moves: Vec<ResolvedSkill>,
    pub changes: Vec<String>,
    pub apply: Option<(String, bool)>,
}

/// Present the resolver's rejection without exposing authoring paths to players.
fn build_error(error: &ContentError, catalog: &ContentCatalog) -> String {
    if let Some(skill) = error
        .message
        .strip_prefix("upgrade requires granted skill ")
        .and_then(|id| ContentId::new(id).ok())
        .and_then(|id| catalog.skill(&id))
    {
        return format!("Requires {} in the resulting build.", skill.name);
    }
    error.message.clone()
}

pub(super) fn inspection(editor: &ActorEditor, catalog: &ContentCatalog) -> Inspection {
    let mut result=Inspection {title:"Battle parameters".into(),description:"Set health, initiative speed, occupied ranks and starting conditions for this encounter.".into(),facts:vec![],moves:vec![],changes:vec![],apply:None};
    let Some(selected) = editor.selection() else {
        return result;
    };
    let from_equipment = matches!(
        selected,
        Selection::EquipmentSkill(_) | Selection::EquipmentAbility(_)
    );
    let current = catalog.resolve_build(&editor.draft.actor.build);
    let candidate = proposed(editor, selected, catalog);
    let next = catalog.resolve_build(&candidate);
    let already_owned = match selected {
        Selection::Weapon(id) => editor.draft.actor.build.weapon == *id,
        Selection::Skill(id) => editor
            .draft
            .actor
            .build
            .skills
            .iter()
            .any(|g| &g.skill == id),
        Selection::Ability(id) => editor.draft.actor.build.abilities.contains(id),
        _ => true,
    };
    let inspected_build = if already_owned {
        current.as_ref().ok()
    } else {
        next.as_ref().ok()
    };
    let mut move_ids = Vec::new();
    match selected {
        Selection::Weapon(id) => {
            if let Some(weapon) = id.as_ref().and_then(|id| catalog.weapon(id)) {
                result.title = weapon.name.clone();
                result.description = weapon.description.clone();
                move_ids = weapon.skills.clone();
                result
                    .facts
                    .push(format!("Equipment kind: {}", weapon.kind));
                for id in &weapon.abilities {
                    if let Some(ability) = catalog.ability(id) {
                        result.facts.push(format!(
                            "Grants Ability · {}: {}",
                            ability.name, ability.description
                        ));
                        result
                            .facts
                            .extend(requirements(&ability.requirements, catalog));
                    }
                }
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
                    "Remove this equipment. Personal selections remain; equipment-dependent selections become inactive.".into();
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
                if id.is_none() || equipped {
                    if id.is_none() && equipped {
                        "Unarmed".into()
                    } else {
                        "Unequip".into()
                    }
                } else {
                    format!("Equip {}", result.title)
                },
                id.is_none() && equipped,
            ));
        }
        Selection::Skill(id) | Selection::EquipmentSkill(id) => {
            if let Some(skill) = catalog.skill(id) {
                result.title = skill.name.clone();
                result.description = skill.description.clone();
                move_ids.push(id.clone());
                let granted = editor
                    .draft
                    .actor
                    .build
                    .skills
                    .iter()
                    .any(|g| g.skill == *id);
                result.facts.push(
                    if from_equipment {
                        "Equipment Skill."
                    } else {
                        "Personal Skill selection."
                    }
                    .into(),
                );
                result
                    .facts
                    .extend(requirements(&skill.requirements, catalog));
                if let Some(reason) =
                    catalog.unmet_requirements(&editor.draft.actor.build, &skill.requirements)
                {
                    result.facts.push(reason);
                }
                if !skill.personal_selectable {
                    result
                        .facts
                        .push("From equipment only; change its source item in Equipment.".into());
                }
                result.apply = Some((
                    if granted { "Remove Skill" } else { "Add Skill" }.into(),
                    !skill.personal_selectable,
                ));
            }
        }
        Selection::Ability(id) | Selection::EquipmentAbility(id) => {
            if let Some(ability) = catalog.ability(id) {
                result.title = ability.name.clone();
                result.description = ability.description.clone();

                result.facts.push(format!(
                    "Passive source: {}. Equipment and Skill prerequisites determine whether it is active.",
                    ability.provenance
                ));
                for upgrade in &ability.upgrades {
                    move_ids.push(upgrade.skill.clone());
                    result.facts.push(format!(
                        "Requires {} from any grant source. {}",
                        catalog
                            .skill(&upgrade.skill)
                            .map_or(upgrade.skill.as_str(), |a| a.name.as_str()),
                        upgrade
                            .operations
                            .iter()
                            .map(operation)
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
                if ability.upgrades.is_empty() {
                    result.facts.push("No prerequisite Skill required.".into());
                }
                result
                    .facts
                    .extend(requirements(&ability.requirements, catalog));
                if let Some(resolved) = inspected_build
                    .and_then(|build| build.abilities.iter().find(|a| a.definition.id == *id))
                {
                    result
                        .facts
                        .push(resolved.inactive_reason.clone().map_or_else(
                            || "Ability active.".into(),
                            |reason| format!("Inactive: {reason}"),
                        ));
                }
                for effect in &ability.effects {
                    match effect {
                    labyrinth_rules::catalog::PassiveEffect::ReduceNegativeStatusDuration { amount } => result.facts.push(format!("New or refreshed negative statuses last {amount} fewer ticks, minimum one. Existing remaining durations are preserved.")),
                }
                }
                if !ability.personal_selectable {
                    result
                        .facts
                        .push("From equipment only; change its source item in Equipment.".into());
                }
                result.apply = Some((
                    if editor.draft.actor.build.abilities.contains(id) {
                        "Remove Ability"
                    } else {
                        "Add Ability"
                    }
                    .into(),
                    !ability.personal_selectable,
                ));
            }
        }
        Selection::Move(id) => {
            move_ids.push(id.clone());
            if let Some(skill) = catalog.skill(id) {
                result.title = skill.name.clone();
                result.description="Effective Skill in your Moveset, including equipment sources and passive upgrades.".into();
            }
        }
    }
    let resolved = inspected_build.or_else(|| current.as_ref().ok());
    if let Some(build) = resolved {
        for id in move_ids {
            if let Some(skill) = build.moveset.skills.iter().find(|a| a.definition.id == id) {
                if !result.moves.iter().any(|a| a.definition.id == id) {
                    result.moves.push(skill.clone());
                }
            } else if let Some(def) = catalog.skill(&id) {
                result.moves.push(ResolvedSkill {
                    definition: def.clone(),
                    grants: vec![],
                    upgrades: vec![],
                });
            }
        }
    }
    if from_equipment {
        result.apply = None;
        let source = editor
            .draft
            .actor
            .build
            .weapon
            .as_ref()
            .and_then(|id| catalog.weapon(id))
            .map_or("equipment", |item| item.name.as_str());
        result.facts.push(format!(
            "From {source}. Change its item on the Equipment page."
        ));
    }
    if !matches!(selected, Selection::Move(_)) && !from_equipment {
        match (&current, &next) {
            (Ok(before), Ok(after)) => result.changes = changes(before, after),
            (_, Err(error)) => {
                result.changes.push(format!(
                    "Cannot apply this choice: {}",
                    build_error(error, catalog)
                ));
                if let Some((_, disabled)) = &mut result.apply {
                    *disabled = true;
                }
            }
            (Err(error), _) => result.changes.push(format!(
                "Current draft needs correction: {}",
                build_error(error, catalog)
            )),
        }
    }
    result
}
