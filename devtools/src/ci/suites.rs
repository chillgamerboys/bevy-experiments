//! Small, project-owned positive suites. Package ownership is not a test filter.

use super::{git, verification, Selection};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const PROCESS: &str =
    "network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight";

/// One explicit group of libtest names. Each filter must discover at least one test.
#[derive(Debug)]
pub struct Suite {
    /// Cargo package owning the tests.
    pub package: &'static str,
    /// Positive libtest name filters; each must execute tests.
    pub filters: &'static [&'static str],
    /// Match complete names instead of module prefixes.
    pub exact: bool,
    /// Explicitly opt into an expensive ignored regression.
    pub ignored: bool,
}

/// Known suites contain literal package/filter names, never executable user arguments.
pub fn get(name: &str) -> Result<Suite, String> {
    let (package, filters, exact, ignored): (_, &[&str], _, _) = match name {
        "labyrinth-all" => ("labyrinth", &[""], false, false),
        "deckbuilder-all" => ("deckbuilder", &[""], false, false),
        "carterfight-all" => ("carterfight", &[""], false, false),
        "labyrinth-ui-model" => ("labyrinth", &["ui::battle::actors::tests::", "ui::battle::history::tests::"], false, false),
        "labyrinth-session" => ("labyrinth", &["session::tests::"], false, false),
        "labyrinth-profile" => ("labyrinth", &["profile::tests::"], false, false),
        "labyrinth-presentation" => ("labyrinth", &["presentation::tests::", "ui::tests::skills::effective_multi_target_forecast_names_every_target_and_conceals_secondary_unknowns"], false, false),
        "labyrinth-scene" => ("labyrinth", &["scene::tests::"], false, false),
        "labyrinth-editor" => ("labyrinth", &["ui::setup::decision_tests::", "ui::setup::tests::"], false, false),
        "labyrinth-lobby" => ("labyrinth", &["ui::shell::lobby::tests::"], false, false),
        "labyrinth-ui-normal" => ("labyrinth", &["normal_1080"], false, false),
        "labyrinth-ui-compatibility" => ("labyrinth", &["compatibility"], false, false),
        "labyrinth-admission" => ("labyrinth", &[
            "network::tests::delivery_order::",
            "network::tests::real_handshake_offer_and_ack_loss_recover_one_peer_from_code_then_profile",
            "network::tests::failed_profile_persistence_never_admits_and_original_invite_can_retry",
            "network::tests::old_attempt_packets_cannot_admit_overwrite_or_disconnect_a_new_reconnect",
        ], false, false),
        "labyrinth-worker" => ("labyrinth", &["network::worker_tests::"], false, false),
        "labyrinth-network-spatial" => ("labyrinth", &["network::tests::spatial::"], false, false),
        "labyrinth-network-requests" => ("labyrinth", &[
            "network::requests::tests::",
            "network::tests::budgets::",
            "network::tests::real_udp_duplicate_and_evicted_replays_never_repeat_an_action",
        ], false, false),
        "labyrinth-network-discovery" => ("labyrinth", &[
            "network::tests::fake_discovery_hands_five_password_joins_to_real_pinned_transport",
            "network::tests::incompatible_and_opaque_service_listings_do_not_open_direct_transport",
        ], true, false),
        "labyrinth-network-gameplay" => ("labyrinth", &[
            "network::tests::real_udp_six_players_reject_seventh_and_wrong_ownership_then_finish_encounter",
            "network::tests::encrypted_custom_build_and_saved_scenario_share_the_live_rules_path",
            "network::tests::incompatible_and_opaque_service_listings_do_not_open_direct_transport",
            "network::tests::queued_old_ui_intent_cannot_be_reinterpreted_as_the_same_heros_next_turn",
            "network::tests::real_udp_fresh_sixth_guest_restores_actor_class_loadout_and_live_combat",
            "network::tests::real_udp_wagon_ownership_reconnects_without_changing_participant_capacity",
        ], true, false),
        "labyrinth-process" => ("labyrinth", &[PROCESS], true, true),
        "deckbuilder-domain" => ("deckbuilder", &["domain::tests::"], false, false),
        "deckbuilder-admission" => ("deckbuilder", &["network::admission_tests::"], false, false),
        "deckbuilder-network" => ("deckbuilder", &["network::tests::"], false, false),
        "deckbuilder-ui-normal" => ("deckbuilder", &["normal_1080"], false, false),
        "deckbuilder-ui-compatibility" => ("deckbuilder", &["compatibility"], false, false),
        "carterfight-rules" => ("carterfight", &[
            "frontend::tests::selection_does_not_resolve_and_displayed_hp_waits_for_its_damage_event",
            "frontend::tests::a_single_advance_reveals_but_does_not_also_acknowledge",
            "frontend::tests::intro_battle_and_outro_preserve_the_complete_local_game",
        ], true, false),
        "carterfight-ui-normal" => ("carterfight", &["normal_1080"], false, false),
        "carterfight-ui-compatibility" => ("carterfight", &["compatibility"], false, false),
        _ => return Err(format!("unknown positive test suite: {name}")),
    };
    Ok(Suite {
        package,
        filters,
        exact,
        ignored,
    })
}

/// Game packages never receive the generic unfiltered package-test fallback.
pub fn game(package: &str) -> bool {
    matches!(package, "labyrinth" | "deckbuilder" | "carterfight")
}

/// Select affected journeys from owner paths and shared boundaries, independently of OS rigor.
pub fn select(selection: &Selection) -> Vec<String> {
    let mut result = BTreeSet::new();
    let mut add = |name: &str| {
        result.insert(name.to_owned());
    };
    let changed = |prefix: &str| selection.paths.iter().any(|p| p.starts_with(prefix));
    let exact = |path: &str| selection.paths.iter().any(|p| p == path);
    let shared_ui = changed("gamekit/ui/") || changed("gamekit/testing/");
    let shared_network = changed("gamekit/multiplayer/") || changed("gamekit/discovery/");
    let testing = matches!(verification::level(selection), Some("testing" | "release"));
    let release = verification::level(selection) == Some("release");
    let release_full = release && selection.full;
    for package in &selection.packages {
        if release_full && game(package) {
            add(&format!("{package}-all"));
            if package == "labyrinth" {
                add("labyrinth-process");
            }
            continue;
        }
        match package.as_str() {
            "labyrinth" => {
                // This baseline is pure authority/projection evidence, with no socket or UI App.
                add("labyrinth-session");
                if release_full
                    || changed("games/labyrinth/src/profile")
                    || changed("games/labyrinth/src/network/admission")
                {
                    add("labyrinth-profile");
                }
                if changed("games/labyrinth/src/presentation") || testing {
                    add("labyrinth-presentation");
                }
                let ui = release_full
                    || changed("games/labyrinth/src/ui")
                    || changed("games/labyrinth/src/scene")
                    || changed("games/labyrinth/assets/")
                    || exact("games/labyrinth/src/lib.rs")
                    || shared_ui;
                if ui {
                    add("labyrinth-ui-normal");
                    add("labyrinth-ui-model");
                    add("labyrinth-presentation");
                    if release {
                        add("labyrinth-ui-compatibility");
                    }
                    if release_full || changed("games/labyrinth/src/ui/setup") || shared_ui {
                        add("labyrinth-editor");
                    }
                    if release_full || changed("games/labyrinth/src/ui/shell") || shared_ui {
                        add("labyrinth-lobby");
                    }
                    if release_full || changed("games/labyrinth/src/scene") || shared_ui {
                        add("labyrinth-scene");
                    }
                }
                let network =
                    release_full || changed("games/labyrinth/src/network") || shared_network;
                if network {
                    add("labyrinth-admission");
                    if release_full
                        || changed("games/labyrinth/src/network/worker")
                        || exact("games/labyrinth/src/network/start.rs")
                    {
                        add("labyrinth-worker");
                    }
                    if release_full
                        || changed("games/labyrinth/src/network/requests")
                        || changed("games/labyrinth/src/network/tests/budgets")
                    {
                        add("labyrinth-network-requests");
                    }
                    if release_full
                        || changed("games/labyrinth/src/network/discovery")
                        || changed("gamekit/discovery/")
                    {
                        add("labyrinth-network-discovery");
                    }
                    if release_full || changed("games/labyrinth/src/network/tests/spatial") {
                        add("labyrinth-network-spatial");
                    }
                    if testing
                        || exact("games/labyrinth/src/network/protocol.rs")
                        || exact("games/labyrinth/src/network/tests.rs")
                    {
                        add("labyrinth-network-gameplay");
                    }
                    if release_full || changed("games/labyrinth/src/network/tests/process") {
                        add("labyrinth-process");
                    }
                }
            }
            "deckbuilder" => {
                add("deckbuilder-domain");
                if release_full || changed("games/deckbuilder/src/network") || shared_network {
                    add("deckbuilder-admission");
                    if testing || exact("games/deckbuilder/src/network.rs") {
                        add("deckbuilder-network");
                    }
                }
                if release_full
                    || exact("games/deckbuilder/src/lib.rs")
                    || changed("games/deckbuilder/assets/")
                    || shared_ui
                {
                    add("deckbuilder-ui-normal");
                    if release {
                        add("deckbuilder-ui-compatibility");
                    }
                }
            }
            "carterfight" => {
                add("carterfight-rules");
                if release_full
                    || changed("games/carterfight/src/frontend/systems")
                    || changed("games/carterfight/src/frontend/tests")
                    || exact("games/carterfight/src/lib.rs")
                    || changed("games/carterfight/assets/")
                    || shared_ui
                {
                    add("carterfight-ui-normal");
                    if release {
                        add("carterfight-ui-compatibility");
                    }
                }
            }
            _ => {}
        }
    }
    result.into_iter().collect()
}

/// Test inventory is evidence of matching only; execution remains a separate required child.
pub fn listed_tests(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_end().ends_with(": test"))
        .count()
}

fn collect_tests(
    stream: proc_macro2::TokenStream,
    prefix: &str,
    output: &mut BTreeMap<String, String>,
) {
    use proc_macro2::{Delimiter, TokenTree};
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut index = 0;
    let mut test = false;
    let mut attributes = String::new();
    while index < tokens.len() {
        if matches!(tokens.get(index), Some(TokenTree::Punct(p)) if p.as_char() == '#') {
            if let Some(TokenTree::Group(group)) = tokens.get(index + 1) {
                if group.delimiter() == Delimiter::Bracket {
                    let attr = group.stream().to_string();
                    test |= attr == "test";
                    attributes.push_str(&attr);
                    index += 2;
                    continue;
                }
            }
        }
        let Some(TokenTree::Ident(kind)) = tokens.get(index) else {
            index += 1;
            continue;
        };
        if kind == "fn" || kind == "mod" {
            if let Some(TokenTree::Ident(name)) = tokens.get(index + 1) {
                let body = tokens
                    .iter()
                    .enumerate()
                    .skip(index + 2)
                    .find(|(_, token)| {
                        matches!(token, TokenTree::Group(g) if g.delimiter() == Delimiter::Brace)
                            || matches!(token, TokenTree::Punct(p) if p.as_char() == ';')
                    });
                if let Some((end, TokenTree::Group(group))) = body {
                    let qualified = if prefix.is_empty() {
                        name.to_string()
                    } else {
                        format!("{prefix}::{name}")
                    };
                    if kind == "mod" {
                        collect_tests(group.stream(), &qualified, output);
                    } else if test {
                        output.insert(qualified, format!("{attributes} {}", group.stream()));
                    }
                    index = end + 1;
                    test = false;
                    attributes.clear();
                    continue;
                }
                test = false;
                attributes.clear();
            }
        }
        index += 1;
    }
}

/// Explicit test bodies, tokenized so comments/whitespace do not create false changes.
pub fn test_bodies(source: &str, module: &str) -> Result<BTreeMap<String, String>, String> {
    let tokens = source
        .parse()
        .map_err(|error| format!("cannot classify Rust tests: {error}"))?;
    let mut result = BTreeMap::new();
    collect_tests(tokens, module, &mut result);
    Ok(result)
}

/// Normalize source while excluding ordinary test-only items. This is a conservative
/// scope hint for delivery, not proof of behavior or developer acceptance.
pub fn production_source(source: &str) -> Result<String, String> {
    fn normalize(stream: proc_macro2::TokenStream) -> String {
        use proc_macro2::{Delimiter, TokenTree};
        let tokens: Vec<_> = stream.into_iter().collect();
        let mut index = 0;
        let mut output = Vec::new();
        while index < tokens.len() {
            if matches!(tokens.get(index), Some(TokenTree::Punct(p)) if p.as_char() == '#') {
                if let Some(TokenTree::Group(group)) = tokens.get(index + 1) {
                    if group.delimiter() == Delimiter::Bracket {
                        let attr = group.stream().to_string().replace(' ', "");
                        if attr == "cfg(test)" || attr == "test" {
                            index += 2;
                            while index < tokens.len() {
                                let end = matches!(tokens.get(index), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
                                    || matches!(tokens.get(index), Some(TokenTree::Punct(p)) if p.as_char() == ';');
                                index += 1;
                                if end {
                                    break;
                                }
                            }
                            continue;
                        }
                        if attr.starts_with("doc=") {
                            index += 2;
                            continue;
                        }
                    }
                }
            }
            output.push(match tokens.get(index) {
                Some(TokenTree::Group(group)) => {
                    format!("{:?}({})", group.delimiter(), normalize(group.stream()))
                }
                Some(token) => token.to_string(),
                None => break,
            });
            index += 1;
        }
        output.join(" ")
    }
    Ok(normalize(source.parse().map_err(|error| {
        format!("cannot classify production input: {error}")
    })?))
}

/// Derive whether this committed batch changes game inputs, ignoring test-only edits.
pub fn gameplay_affected(root: &Path, selection: &Selection) -> Option<bool> {
    if selection.full && selection.paths.is_empty() {
        return None;
    }
    for path in &selection.paths {
        let game_input = path.starts_with("games/") || path.starts_with("gamekit/");
        let compiled = selection
            .reasons
            .iter()
            .any(|reason| reason.starts_with(&format!("{path}: compiled input")));
        if !game_input || (path.ends_with(".md") && !compiled) {
            continue;
        }
        if path.contains("/tests/") || path.ends_with("_tests.rs") || path.ends_with("/tests.rs") {
            continue;
        }
        if !path.ends_with(".rs") {
            return Some(true);
        }
        let after = git(root, &["show", &format!("{}:{path}", selection.head)]).ok();
        let before = selection
            .base
            .as_ref()
            .and_then(|base| git(root, &["show", &format!("{base}:{path}")]).ok());
        let normalized =
            |source: Option<String>| source.map(|source| production_source(&source)).transpose();
        match (normalized(before), normalized(after)) {
            (Ok(before), Ok(after)) if before == after => {}
            _ => return Some(true),
        }
    }
    if selection.paths.iter().any(|path| {
        matches!(
            path.as_str(),
            "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml" | "rust-toolchain"
        )
    }) {
        None // Dependency/toolchain impact needs the delivery owner's classification.
    } else {
        Some(false)
    }
}

fn file_module(path: &str) -> String {
    match path {
        "games/labyrinth/src/ui/setup_tests.rs" => return "ui::setup::tests".into(),
        "games/labyrinth/src/ui/setup/tests.rs" => return "ui::setup::decision_tests".into(),
        "games/labyrinth/src/ui/shell/lobby_tests.rs" => return "ui::shell::lobby::tests".into(),
        _ => {}
    }
    path.split_once("/src/")
        .map_or("", |(_, path)| path)
        .trim_end_matches(".rs")
        .split('/')
        .filter(|part| !matches!(*part, "lib" | "mod"))
        .collect::<Vec<_>>()
        .join("::")
}

fn matches_suite(suite: &Suite, test: &str) -> bool {
    suite.filters.iter().any(|filter| {
        if suite.exact {
            test == *filter
        } else {
            test.contains(filter)
        }
    })
}

/// New/changed tests must be covered or explicitly deferred display cases. Unknown
/// source boundaries require a small mapping decision instead of silently disappearing.
pub fn validate_changes(root: &Path, selection: &Selection) -> Result<(), String> {
    for path in &selection.paths {
        let Some(rest) = path.strip_prefix("games/") else {
            continue;
        };
        let Some((package, relative)) = rest.split_once('/') else {
            continue;
        };
        if !game(package) || !path.ends_with(".rs") {
            continue;
        }
        let known = match package {
            "labyrinth" => {
                ["src/ui/", "src/session/", "src/scene/", "rules/"]
                    .iter()
                    .any(|p| relative.starts_with(p))
                    || matches!(
                        relative,
                        "src/lib.rs"
                            | "src/main.rs"
                            | "src/ui/mod.rs"
                            | "src/scene.rs"
                            | "src/session.rs"
                            | "src/presentation.rs"
                            | "src/profile.rs"
                            | "src/view.rs"
                            | "src/network/mod.rs"
                            | "src/network/start.rs"
                            | "src/network/admission.rs"
                            | "src/network/protocol.rs"
                            | "src/network/requests.rs"
                            | "src/network/discovery.rs"
                            | "src/network/worker_tests.rs"
                            | "src/network/tests.rs"
                            | "src/network/tests/budgets.rs"
                            | "src/network/tests/delivery_order.rs"
                            | "src/network/tests/process.rs"
                            | "src/network/tests/spatial.rs"
                    )
                    || relative.starts_with("examples/")
            }
            "deckbuilder" => matches!(
                relative,
                "src/lib.rs"
                    | "src/main.rs"
                    | "src/domain.rs"
                    | "src/network.rs"
                    | "src/network/admission.rs"
                    | "src/network/admission_tests.rs"
                    | "src/network/admission_tests/delivery_order.rs"
            ),
            "carterfight" => {
                relative.starts_with("src/backend/")
                    || relative.starts_with("src/frontend/")
                    || matches!(relative, "src/lib.rs" | "src/main.rs")
            }
            _ => false,
        };
        if !known {
            return Err(format!(
                "classify the affected test suite for new game boundary {path}"
            ));
        }
        if relative.starts_with("rules/") || !relative.starts_with("src/") {
            continue;
        }
        let Ok(source) = git(root, &["show", &format!("{}:{path}", selection.head)]) else {
            continue;
        };
        let module = file_module(path);
        let before = selection
            .base
            .as_ref()
            .and_then(|base| git(root, &["show", &format!("{base}:{path}")]).ok())
            .map(|source| test_bodies(&source, &module))
            .transpose()?
            .unwrap_or_default();
        for (name, body) in test_bodies(&source, &module)? {
            if before.get(&name) == Some(&body) {
                continue;
            }
            let covered = selection
                .suites
                .iter()
                .filter_map(|name| get(name).ok())
                .any(|suite| suite.package == package && matches_suite(&suite, &name));
            let display_deferred =
                name.contains("compatibility") && verification::level(selection) != Some("release");
            if !covered && !display_deferred {
                return Err(format!("changed test {package}::{name} is outside selected suites; classify it before accepting coverage"));
            }
        }
    }
    Ok(())
}

/// Count actual libtest successes, not a listing or ignored-test count.
pub fn passed_tests(line: &str) -> usize {
    line.strip_prefix("test result: ok. ")
        .and_then(|value| value.split_once(" passed;"))
        .and_then(|(count, _)| count.parse().ok())
        .unwrap_or(0)
}

/// Execute one positive suite, refusing Cargo's successful zero-match result.
pub fn run(root: &Path, name: &str) -> Result<usize, String> {
    let suite = get(name)?;
    let mut total = 0;
    for filter in suite.filters {
        let mut args = vec![
            "test",
            "--locked",
            "-p",
            suite.package,
            "--profile",
            "ci",
            "--lib",
            filter,
            "--",
        ];
        if suite.exact {
            args.push("--exact");
        }
        if suite.ignored {
            args.push("--ignored");
        }
        if matches!(name, "labyrinth-editor" | "labyrinth-lobby") {
            args.extend(["--skip", "compatibility", "--skip", "normal_1080"]);
        }
        let listed = Command::new("cargo")
            .args(&args)
            .arg("--list")
            .current_dir(root)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;
        if !listed.status.success() {
            return Err(format!("suite {name} inventory failed: {}", listed.status));
        }
        let count = listed_tests(&String::from_utf8(listed.stdout).map_err(|e| e.to_string())?);
        if count == 0 {
            return Err(format!("suite {name} filter {filter:?} matched no tests"));
        }
        writeln!(
            std::io::stderr().lock(),
            "+ suite {name}: {count} tests for {filter}"
        )
        .map_err(|e| e.to_string())?;
        let mut child = Command::new("cargo")
            .args(&args)
            .args(["--nocapture", "--test-threads=1"])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut executed = 0;
        for line in BufReader::new(child.stdout.take().ok_or("missing suite stdout")?).lines() {
            let line = line.map_err(|e| e.to_string())?;
            writeln!(std::io::stdout().lock(), "{line}").map_err(|e| e.to_string())?;
            executed += passed_tests(&line);
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("suite {name} failed: {status}"));
        }
        if executed == 0 {
            return Err(format!("suite {name} executed no passing tests"));
        }
        total += executed;
    }
    Ok(total)
}
