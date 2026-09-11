# Tests and evidence

From the repository root, with Python 3.11 or newer for the standard-library tools:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo deny check
python3 scripts/check_repo.py
python3 scripts/check_distribution.py
python3 -m unittest discover -s scripts/tests -v
python3 skills/scripts/validate_skills.py
python3 skills/scripts/validate_gameskills.py
python3 -m unittest discover -s skills/tests -v
```

Use `cargo test -p <package> --profile ci` for focused iteration. The list above is
the broad validation set, not a requirement to rebuild all games for narrative
documentation. `cargo deny` is a separately installed dependency-policy tool.

## CI selection

[The workflow](../.github/workflows/gamekit.yml) always checks repository layout,
local links and the routing regressions, then selects component jobs through
[scripts/ci.py](../scripts/ci.py). Narrative docs avoid Rust and skill-runtime
jobs; skill instructions and tooling receive the Python/skill matrix; a game edit
receives its package tests and Clippy. Shared libraries also select their reverse
consumers and applicable distribution, minimal-feature and browser-core checks.
Selected Rust and skill jobs retain macOS/Linux/Windows coverage. Python repository
tool tests also run on those platforms with the skills job when tooling changes.

The selector reads committed base and tested-tree manifests, including normal,
development, build and target-specific local dependencies. PR checks compare the
event's base commit with the tested GitHub merge tree; main pushes compare their
before/after commits. Renames and deletions retain old owners. Root dependency,
workflow/classifier and unknown-input changes select the full suite. Static Rust
file inclusions count as build inputs; dynamic includes/build scripts prevent the
documentation shortcut. Each run's summary reports selected packages and reasons.

The final `ci` job rejects failed, cancelled, missing or unexpectedly skipped
selected jobs. It does not alter repository branch-protection settings. A manual
Gamekit workflow dispatch runs the full suite; use that on release candidates.
Labyrinth changes retain the six-process restart regression. Finer exclusions
inside its networking/UI source are deferred until their inputs are mapped.

To inspect selection locally, supply full committed object IDs:

```sh
python3 scripts/ci.py select --base BASE_COMMIT --head HEAD_COMMIT
python3 scripts/ci.py select --full
```

Selection reads committed objects, not uncommitted edits, and runs no game checks.
Structural skill tests do not replace bounded native/model evaluations of changed
guidance; those remain separately recorded acceptance evidence.

`check_distribution.py` asks Cargo for each library package's file list, rejects
escapes/symlinks, and stages those sources without `games/`. It builds an unrelated
consumer using empty, pure-algorithm, UI and native-networking feature selections,
checking the activated dependency graph for game or networking leakage. Use
`--case empty|pure|ui|network` for a focused probe. Temporary sources are deleted;
artifacts reuse `target/`. This proves independent source consumption, not registry
publication, a complete game, visual quality or cross-machine networking. The native
probe does not open sockets or invoke Tailscale. CI runs all four cases on three OSes.

| Claim | Evidence | Does not establish |
|---|---|---|
| Rules and deterministic transitions | Pure domain tests | Plugin wiring |
| Ordering, focus, typed intent | Production-plugin App/input tests | Rendered appearance |
| Hierarchy, labels, reachability | Structural/layout checks | Pixel contrast or motion |
| Presentation | Deterministic rendered frames | Interaction or authority |
| Input, resizing, scrolling, motion | Interactive walk | Exhaustive rules correctness |
| Restart recovery | Fresh guest loads credential and rejoins live host | Host restart recovery |
| LAN/tailnet operation | Distinct-machine discovery, join and gameplay | Steam integration |

Test listing, route resolution, encryption, admission, disclosure and gameplay
separately. Fake providers and localhost sockets do not prove multicast/firewalls.
Keep missing cross-machine evidence explicit.

UI gates cover 1280x720, 1920x1080 and 3840x2160 at Auto and 200% semantic scale.
Combine structure, captures and pointer/keyboard/modal/resize/motion review. Keep
current captures under `target/review/` through review, then retain a concise
revision/command/result record instead of unlimited generated artifacts.
For Labyrinth's current description/dock correction, manual acceptance prioritizes
normal-scale play; the 200% visual pass is deferred, not a blocker. Existing
automated scale regressions are retained.

## Context, forecasts and unknown information

The shared contextual-help tests are independent of Labyrinth. They exercise
focus/pointer precedence, modal scope/restoration, hidden/disabled/removed sources,
clipping, activation non-interference and unchanged-resource detection. Their
explicit `Interaction` and geometry fixtures prove selection mechanics, not native
cursor hit testing or rendered tooltip placement. Each adopter must also exercise
real input and layout through its production plugin stack; helpful text appearing
does not prove that the associated action can be selected and confirmed.

Keep Labyrinth's forecast evidence at two separate levels:

- Pure rules: base power versus effective damage and actual HP loss, shared
  immediate resolution, status application/removal, position changes, no mutation
  or random/turn advancement, and off-turn previews granting no commit authority.
- Presentation: known versus unknown HP, modifiers and details; uncertainty text
  and absent exact projections; conditional periodic-effect explanations; and
  matching disclosure in labels, inspection, logs and contextual information.

Vary concealed inputs while keeping public facts fixed and compare the resulting
presentation, including error shape and derived values. Test partial disclosure,
not only an entirely concealed actor. Separately review projected HP segments,
pending effect markers and confirmation clarity at all supported canvas sizes.
Normal encounters remain fully revealed: a hidden-information fixture is neither
an implemented reveal ability nor evidence that network payloads are filtered.

No automated selection test, snapshot or forecast parity check establishes the
feel of the dock. A pointer/keyboard walk still checks hover-to-focus transitions,
off-turn inspection, ability -> target -> Confirm, modal return, overflow and
resizing. Record any missing interactive or cross-machine evidence explicitly.

Tooltip lifecycle regressions include immediate first-frame preview and departure,
one-second continuous hover to lock, persistence over empty space, source switching,
explicit keyboard inspection, modal cleanup, and ×/Escape/outside dismissal.
The native-layout test compares the card rectangle on every frame across locking:
the preview must use the same shorter geometry as the locked card, not reserve an
extra footer. Native pointer tests close the × over an underlying character and
verify that stationary-pointer dismissal does not reveal a new tooltip. Render
`labyrinth_review ... 1280 720 auto help` and `help-locked` for separate authored
presentation states; those captures freeze timing and do not prove hover duration.

See [Labyrinth](../games/labyrinth/labyrinth-testing.md),
[Carterfight](../games/carterfight/README.md) and
[deckbuilder](../games/deckbuilder_ui/README.md) for game-specific acceptance.
Skill validation proves structure/rendering parity, not agent selection behavior.
