//! One character workspace; only its browser and details scroll.
use super::*;
use bevy_gamekit::ui::{UiDisabled, UiFocusId};

#[derive(Component)]
struct EditorRoot;
#[derive(Component)]
struct EditorNotice;
#[derive(Component)]
struct EditorSave;
#[derive(Component)]
struct EditorDirty;
#[derive(Component)]
struct BrowserScroll(Category);

fn editable(view: &LabyrinthView, id: ActorId) -> bool {
    view.mode == ViewMode::Lobby
        && view.admitted
        && (view.host
            || view
                .company
                .iter()
                .any(|m| m.actor == id && Some(m.owner) == view.player))
}
fn box_node() -> Node {
    Node {
        min_width: Val::Px(0.),
        min_height: Val::Px(0.),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.),
        ..default()
    }
}
fn paragraph(
    world: &mut World,
    parent: Entity,
    key: &str,
    text: impl Into<String>,
    role: UiTextRole,
) -> Entity {
    let entity = label(world, parent, key, text, role);
    world.entity_mut(entity).insert(Node {
        max_width: Val::Percent(100.),
        flex_shrink: 0.,
        ..default()
    });
    entity
}
fn button(
    world: &mut World,
    parent: Entity,
    key: impl Into<String>,
    text: impl Into<String>,
    action: SetupAction,
    disabled: bool,
) -> Entity {
    let entity = control(world, parent, key, text, Action::Setup(action), disabled);
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        node.flex_shrink = 0.;
        node.min_width = Val::Px(0.);
    }
    entity
}
fn row(world: &mut World, parent: Entity, key: &str) -> Entity {
    column(
        world,
        parent,
        key,
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.),
            min_width: Val::Px(0.),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            ..default()
        },
    )
}

pub(super) fn present(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    if view.mode != ViewMode::Lobby || !view.admitted {
        ui.editor = None;
    }
    if ui.editor.as_ref().is_some_and(|editor| {
        editor.pending_save
            && editor
                .submitted_revision
                .is_some_and(|revision| view.revision > revision)
            && view.setup_revision >= editor.revision
            && view
                .catalog
                .as_ref()
                .zip(view.scenario.as_ref())
                .is_some_and(|(catalog, scenario)| {
                    editor
                        .validated(catalog)
                        .is_ok_and(|draft| actor(scenario, editor.id) == Some(&draft))
                })
    }) {
        ui.editor = None;
    }
    let Some(editor) = &mut ui.editor else {
        despawn_marked::<EditorRoot>(world);
        return;
    };
    let Some(catalog) = &view.catalog else {
        return;
    };
    editor.ensure_selection(catalog);
    let metrics = *world.resource::<ResolvedUiMetrics>();
    let compact = metrics.logical_size.x / metrics.content_scale < 1100.;
    if editor.compact != compact {
        editor.compact = compact;
        editor.generation += 1;
    }
    let can_edit = editable(view, editor.id);
    let conflict = view.setup_revision != editor.revision;
    let notice = if conflict {
        "Setup changed. Discard this draft to reload the latest character.".to_owned()
    } else if !can_edit {
        "Viewing only. This character is assigned to another player.".into()
    } else {
        editor
            .error
            .as_deref()
            .or(view.notice.as_deref())
            .unwrap_or("")
            .to_owned()
    };
    for mut text in world
        .query_filtered::<&mut Text, With<EditorNotice>>()
        .iter_mut(world)
    {
        text.0.clone_from(&notice);
    }
    let dirty = if editor.pending_save {
        "Applying draft · waiting for host acknowledgment"
    } else if editor.dirty() {
        "Unapplied draft · battle setup is unchanged"
    } else {
        "Saved character · inspect a choice before changing it"
    };
    for mut text in world
        .query_filtered::<&mut Text, With<EditorDirty>>()
        .iter_mut(world)
    {
        text.0 = dirty.into();
    }
    let controls = world
        .query_filtered::<Entity, With<EditorSave>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in controls {
        set_disabled(world, entity, !can_edit || conflict || editor.pending_save);
    }
    if editor.mounted == Some(editor.generation) {
        return;
    }
    for (scope, scroll) in world
        .query::<(&BrowserScroll, &ScrollPosition)>()
        .iter(world)
    {
        editor.scrolls.insert(scope.0, scroll.y);
    }
    despawn_marked::<EditorRoot>(world);
    editor.mounted = Some(editor.generation);
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let root = world
        .spawn((
            bevy_gamekit::ui::menu_overlay("Character Editor"),
            EditorRoot,
        ))
        .id();
    let panel = world
        .spawn((
            bevy_gamekit::ui::menu_panel("Character Build"),
            ChildOf(root),
        ))
        .id();
    world.entity_mut(panel).insert(Node {
        width: Val::Percent(98.),
        height: Val::Percent(96.),
        max_width: Val::Px(1680.),
        max_height: Val::Percent(98.),
        overflow: Overflow::clip(),
        padding: UiRect::all(Val::Px(16.)),
        ..box_node()
    });
    let header = row(world, panel, "Character Identity");
    let identity = column(
        world,
        header,
        "Identity Text",
        Node {
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            ..box_node()
        },
    );
    paragraph(
        world,
        identity,
        "Build Title",
        &editor.name,
        UiTextRole::Title,
    );
    let (team, start, end) = details::rank_span(view, editor);
    let weapon = editor
        .draft
        .actor
        .build
        .weapon
        .as_ref()
        .and_then(|id| catalog.weapon(id))
        .map_or("Unarmed", |w| w.name.as_str());
    let owner = if team == labyrinth_rules::Team::Enemies {
        "Host-controlled enemy".into()
    } else {
        view.company
            .iter()
            .find(|m| m.actor == editor.id)
            .and_then(|member| view.players.iter().find(|p| p.slot == member.owner))
            .map_or_else(
                || "Unassigned".into(),
                |p| format!("Controlled by {}", p.name),
            )
    };
    paragraph(
        world,
        identity,
        "Character Context",
        format!(
            "{} · ranks {start}–{end} · {weapon} · {owner}",
            if team == labyrinth_rules::Team::Heroes {
                "Party"
            } else {
                "Enemy"
            }
        ),
        UiTextRole::Supporting,
    );
    let roster = view
        .scenario
        .as_ref()
        .map(|s| {
            s.heroes
                .iter()
                .chain(&s.enemies)
                .map(|a| a.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(index) = roster.iter().position(|id| *id == editor.id) {
        if let Some(id) = index.checked_sub(1).and_then(|i| roster.get(i)) {
            button(
                world,
                header,
                "Previous Character",
                "Previous",
                SetupAction::Edit(*id),
                false,
            );
        }
        if let Some(id) = roster.get(index + 1) {
            button(
                world,
                header,
                "Next Character",
                "Next",
                SetupAction::Edit(*id),
                false,
            );
        }
    }
    let nav = row(world, panel, "Character Categories");
    if let Some(mut node) = world.get_mut::<Node>(nav) {
        node.overflow = Overflow::scroll_x();
    }
    for category in Category::ALL {
        let entity = button(
            world,
            nav,
            format!("Category {}", category.name()),
            category.name(),
            SetupAction::Category(category),
            false,
        );
        world
            .entity_mut(entity)
            .insert(appearance.control(editor.category == category));
    }
    let workspace = column(
        world,
        panel,
        "Character Workspace",
        Node {
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.),
            overflow: Overflow::clip(),
            ..box_node()
        },
    );
    if editor.pending_exit.is_some() {
        let confirmation = column(
            world,
            workspace,
            "Discard Confirmation",
            Node {
                width: Val::Percent(100.),
                padding: UiRect::all(Val::Px(24.)),
                ..box_node()
            },
        );
        paragraph(
            world,
            confirmation,
            "Discard Question",
            "Discard this character's unapplied changes?",
            UiTextRole::Title,
        );
        paragraph(
            world,
            confirmation,
            "Discard Explanation",
            "Your saved character will be kept. Choose Keep editing to return to this draft.",
            UiTextRole::Body,
        );
        button(
            world,
            confirmation,
            "Keep Editing",
            "Keep editing",
            SetupAction::KeepEditing,
            false,
        );
        button(
            world,
            confirmation,
            "Confirm Discard",
            if matches!(editor.pending_exit, Some(ExitTarget::Actor(_))) {
                "Discard draft and switch character"
            } else {
                "Discard draft and close"
            },
            SetupAction::ConfirmDiscard,
            false,
        );
    } else {
        if !compact || !editor.detail_only {
            let browser = column(
                world,
                workspace,
                "Character Browser",
                Node {
                    width: if compact {
                        Val::Percent(100.)
                    } else {
                        Val::Percent(37.)
                    },
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::right(Val::Px(12.)),
                    ..box_node()
                },
            );
            world.entity_mut(browser).insert((
                BrowserScroll(editor.category),
                ScrollPosition(Vec2::new(
                    0.,
                    editor.scrolls.get(&editor.category).copied().unwrap_or(0.),
                )),
            ));
            mount_browser(world, browser, editor, catalog, view, can_edit);
        }
        if !compact || editor.detail_only {
            let inspector = column(
                world,
                workspace,
                "Selection Inspector",
                Node {
                    flex_grow: 1.,
                    flex_basis: Val::Px(0.),
                    ..box_node()
                },
            );
            mount_inspector(
                world,
                inspector,
                editor,
                catalog,
                (start, end),
                can_edit,
                compact,
            );
        }
    }
    let footer = column(
        world,
        panel,
        "Build Footer",
        Node {
            flex_shrink: 0.,
            border: UiRect::top(Val::Px(1.)),
            padding: UiRect::top(Val::Px(10.)),
            ..box_node()
        },
    );
    world
        .entity_mut(footer)
        .insert(BorderColor::all(appearance.line));
    let dirty_text = paragraph(world, footer, "Draft Status", dirty, UiTextRole::Supporting);
    world.entity_mut(dirty_text).insert(EditorDirty);
    let notice_entity = paragraph(
        world,
        footer,
        "Build Notice",
        notice,
        UiTextRole::Supporting,
    );
    world.entity_mut(notice_entity).insert(EditorNotice);
    let actions = row(world, footer, "Build Actions");
    let apply = button(
        world,
        actions,
        "Apply Build",
        "Apply build",
        SetupAction::Save,
        !can_edit || conflict || editor.pending_save || editor.pending_exit.is_some(),
    );
    world
        .entity_mut(apply)
        .insert((EditorSave, appearance.control(true)));
    button(
        world,
        actions,
        "Reload Build",
        "Discard draft",
        SetupAction::Reload,
        false,
    );
    button(
        world,
        actions,
        "Close Build",
        "Close editor",
        SetupAction::Cancel,
        false,
    );
}

fn browser_row(
    world: &mut World,
    parent: Entity,
    editor: &ActorEditor,
    key: String,
    title: String,
    description: String,
    selection: Selection,
) {
    let selected = editor.selection() == Some(&selection);
    let entity = button(
        world,
        parent,
        key.clone(),
        title,
        SetupAction::Inspect(selection),
        false,
    );
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    world.entity_mut(entity).insert((
        Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            row_gap: Val::Px(5.),
            flex_shrink: 0.,
            padding: UiRect::all(Val::Px(10.)),
            border: UiRect::left(Val::Px(3.)),
            ..default()
        },
        appearance.control(selected),
        UiFocusId::new("labyrinth-character", format!("{}:{key}", editor.id.0)),
    ));
    paragraph(
        world,
        entity,
        "Choice Description",
        description,
        UiTextRole::Supporting,
    );
}
fn mount_browser(
    world: &mut World,
    parent: Entity,
    editor: &ActorEditor,
    catalog: &ContentCatalog,
    view: &LabyrinthView,
    can_edit: bool,
) {
    paragraph(
        world,
        parent,
        "Browser Heading",
        editor.category.name(),
        UiTextRole::Title,
    );
    paragraph(
        world,
        parent,
        "Browse Instruction",
        "Select to inspect. Changes stay in your draft until Apply build.",
        UiTextRole::Supporting,
    );
    match editor.category {
        Category::Equipment => {
            browser_row(
                world,
                parent,
                editor,
                "Weapon None".into(),
                "Unarmed".into(),
                "Keep innate and learned moves without weapon grants.".into(),
                Selection::Weapon(None),
            );
            for weapon in &catalog.definition().weapons {
                let equipped = editor.draft.actor.build.weapon.as_ref() == Some(&weapon.id);
                let description = weapon
                    .grants
                    .iter()
                    .filter_map(|id| catalog.ability(id))
                    .map(|a| format!("{}: {}", a.name, details::move_summary(a)))
                    .collect::<Vec<_>>()
                    .join("\n");
                browser_row(
                    world,
                    parent,
                    editor,
                    format!("Weapon {}", weapon.id),
                    format!(
                        "{}{}",
                        weapon.name,
                        if equipped { " · equipped" } else { "" }
                    ),
                    description,
                    Selection::Weapon(Some(weapon.id.clone())),
                );
            }
        }
        Category::Innate => {
            for ability in &catalog.definition().abilities {
                let granted = editor
                    .draft
                    .actor
                    .build
                    .innate
                    .iter()
                    .any(|g| g.ability == ability.id);
                browser_row(
                    world,
                    parent,
                    editor,
                    format!("Innate {}", ability.id),
                    format!(
                        "{}{}",
                        ability.name,
                        if granted { " · in draft" } else { "" }
                    ),
                    details::move_summary(ability),
                    Selection::Innate(ability.id.clone()),
                );
            }
        }
        Category::Learned => {
            for skill in &catalog.definition().learned_skills {
                browser_row(
                    world,
                    parent,
                    editor,
                    format!("Learned {}", skill.id),
                    format!(
                        "{}{}",
                        skill.name,
                        if editor.draft.actor.build.learned_skills.contains(&skill.id) {
                            " · learned"
                        } else {
                            ""
                        }
                    ),
                    skill.description.clone(),
                    Selection::Learned(skill.id.clone()),
                );
            }
        }
        Category::Moves => match catalog.resolve_build(&editor.draft.actor.build) {
            Ok(build) => {
                for ability in &build.abilities {
                    browser_row(
                        world,
                        parent,
                        editor,
                        format!("Result Move {}", ability.definition.id),
                        ability.definition.name.clone(),
                        details::move_summary(&ability.definition),
                        Selection::Move(ability.definition.id.clone()),
                    );
                }
            }
            Err(error) => {
                paragraph(
                    world,
                    parent,
                    "Invalid Build",
                    error.to_string(),
                    UiTextRole::Body,
                );
            }
        },
        Category::Parameters => {
            paragraph(world,parent,"Parameter Explanation","Battle values and initial conditions. Changing them does not introduce character progression.",UiTextRole::Supporting);
            for (title, field, value, max) in [
                ("Name", BuildField::Name, &editor.name, 128),
                ("Maximum HP", BuildField::MaxHp, &editor.max_hp, 5),
                ("Speed", BuildField::Speed, &editor.speed, 5),
                (
                    "Formation spaces",
                    BuildField::Footprint,
                    &editor.footprint,
                    1,
                ),
                (
                    "Starting HP (blank = full)",
                    BuildField::StartingHp,
                    &editor.starting_hp,
                    5,
                ),
            ] {
                paragraph(
                    world,
                    parent,
                    &format!("Build {field:?} Label"),
                    title,
                    UiTextRole::Body,
                );
                let bundle =
                    bevy_gamekit::ui::text_field(world.resource::<UiFonts>(), title, value, max);
                let entity = world
                    .spawn((
                        bundle,
                        UiSkin::Field,
                        Field::Build(field),
                        UiFocusId::new("labyrinth-build", format!("{}:{field:?}", editor.id.0)),
                        ChildOf(parent),
                    ))
                    .id();
                if !can_edit || (matches!(field, BuildField::Footprint) && !view.host) {
                    world.entity_mut(entity).insert(UiDisabled);
                }
            }
            paragraph(
                world,
                parent,
                "Starting Status Title",
                "Starting conditions",
                UiTextRole::Body,
            );
            for kind in [
                labyrinth_rules::StatusKind::Bleed,
                labyrinth_rules::StatusKind::Brace,
                labyrinth_rules::StatusKind::Haste,
                labyrinth_rules::StatusKind::Weakened,
            ] {
                let present = editor
                    .draft
                    .starting_statuses
                    .iter()
                    .any(|s| s.kind == kind);
                button(
                    world,
                    parent,
                    format!("Starting {kind:?}"),
                    format!(
                        "{} {}",
                        if present { "Remove" } else { "Add" },
                        labyrinth_rules::status_definition(kind).name
                    ),
                    SetupAction::Status(kind),
                    !can_edit,
                );
                paragraph(
                    world,
                    parent,
                    "Condition Explanation",
                    labyrinth_rules::status_definition(kind).description,
                    UiTextRole::Supporting,
                );
            }
            paragraph(
                world,
                parent,
                "Preset Heading",
                "Start from a character preset",
                UiTextRole::Body,
            );
            for preset in &catalog.definition().actor_presets {
                browser_row(
                    world,
                    parent,
                    editor,
                    format!("Build Preset {}", preset.id),
                    preset.name.clone(),
                    format!(
                        "{} HP · speed {} · {} formation spaces",
                        preset.max_hp, preset.base_speed, preset.footprint
                    ),
                    Selection::Preset(preset.id.clone()),
                );
            }
        }
    }
}
fn mount_inspector(
    world: &mut World,
    parent: Entity,
    editor: &ActorEditor,
    catalog: &ContentCatalog,
    span: (u8, u8),
    can_edit: bool,
    compact: bool,
) {
    let info = details::inspection(editor, catalog);
    let top = row(world, parent, "Inspection Actions");
    if compact {
        button(
            world,
            top,
            "Back To Choices",
            "Back to choices",
            SetupAction::Browse,
            false,
        );
    }
    if let Some((title, disabled)) = &info.apply {
        button(
            world,
            top,
            "Apply Inspected Choice",
            title,
            SetupAction::ApplyInspected,
            *disabled || !can_edit,
        );
    }
    let content = column(
        world,
        parent,
        "Inspected Choice Scroll",
        Node {
            flex_grow: 1.,
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(Val::Px(12.)),
            ..box_node()
        },
    );
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    world
        .entity_mut(content)
        .insert(BackgroundColor(appearance.dock));
    paragraph(
        world,
        content,
        "Inspected Choice Title",
        &info.title,
        UiTextRole::Title,
    );
    paragraph(
        world,
        content,
        "Inspected Description",
        &info.description,
        UiTextRole::Supporting,
    );
    for fact in info.facts {
        paragraph(world, content, "Choice Fact", fact, UiTextRole::Supporting);
    }
    if !info.changes.is_empty() {
        paragraph(
            world,
            content,
            "Comparison Heading",
            "If applied to this draft",
            UiTextRole::Body,
        );
        for change in info.changes {
            paragraph(
                world,
                content,
                "Build Change",
                change,
                UiTextRole::Supporting,
            );
        }
    }
    for ability in info.moves {
        paragraph(
            world,
            content,
            "Effective Move Name",
            &ability.definition.name,
            UiTextRole::Body,
        );
        for fact in details::move_facts(&ability, span, catalog) {
            paragraph(
                world,
                content,
                "Effective Move Fact",
                fact,
                UiTextRole::Supporting,
            );
        }
    }
    if matches!(editor.category, Category::Parameters) && editor.selection().is_none() {
        paragraph(world,content,"Parameters Guide","HP is health at the start of battle. Speed contributes to the initiative roll. Formation spaces determine the ranks occupied by this actor. Starting HP may be blank for full health. Conditions use the existing combat rules.",UiTextRole::Body);
    }
}
