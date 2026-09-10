# Tests and evidence

From the repository root, with Python 3.11 or newer for the standard-library tools:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo deny check
python3 scripts/check_repo.py
python3 -m unittest discover -s scripts/tests -v
python3 skills/scripts/validate_skills.py
python3 -m unittest discover -s skills/tests -v
```

Use `cargo test -p <package> --profile ci` for focused iteration. Preserve three-OS
CI, minimal-feature capability checks and browser-core compile checks. `cargo deny`
is a separately installed dependency-policy tool.

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

See [Labyrinth](../games/labyrinth/labyrinth-testing.md),
[Carterfight](../games/carterfight/README.md) and
[deckbuilder](../games/deckbuilder_ui/README.md) for game-specific acceptance.
Skill validation proves structure/rendering parity, not agent selection behavior.
