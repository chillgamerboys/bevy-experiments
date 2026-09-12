# CI selection and verification


[The workflow](../../.github/workflows/gamekit.yml) always checks repository layout,
local links and the routing regressions, then selects component jobs through
[`repo-devtools ci`](../src/ci/mod.rs). Narrative docs avoid Rust and skill-runtime
jobs; skill instructions receive the skill matrix with Rust structural validators
and Rust CLI tests; a game edit
receives its package tests and Clippy. Shared libraries also select their reverse
consumers and applicable distribution, minimal-feature and browser-core checks.
Selected Rust and skill jobs retain macOS/Linux/Windows coverage. The Rust routing,
command and gate regressions also run on those platforms with the skills job when
tooling changes.
Rust tool packages receive their focused Cargo checks. Repository validator implementation
changes also select skill validation and external consumer checks. CI implementation,
its tests, and its shared library/CLI/input entrypoints select the full suite. Reference verification and Cargo packaging run only when their
respective tool package is selected or the full suite is required. All use the same
pinned toolchain as game builds. The always-run layout check bootstraps only the small
repository-tool package. The final gate independently builds that same checked-out
Rust package with the locked dependencies and pinned toolchain; a bootstrap failure
fails its job, with no older binary or successful fallback. CI requires no Python
setup or interpreter.

The selector reads committed base and tested-tree manifests, including normal,
development, build and target-specific local dependencies. PR checks compare the
event's base commit with the tested GitHub merge tree; main pushes compare their
before/after commits. Renames and deletions retain old owners. Root dependency,
workflow/classifier and unknown-input changes select the full suite. Static Rust
file inclusions count as build inputs alongside the file's normal owner; dynamic
or unsupported includes/build scripts select the full suite. Each run's summary
reports selected packages and reasons.

The final `ci` job rejects failed, cancelled, missing or unexpectedly skipped
selected jobs. It does not alter repository branch-protection settings. A manual
Gamekit workflow dispatch runs the full suite; use that on release candidates.
Labyrinth changes retain the six-process restart regression. Finer exclusions
inside its networking/UI source are deferred until their inputs are mapped.

To inspect selection locally, supply full committed object IDs:

```sh
cargo run --locked -p repo-devtools --profile ci -- ci select --base BASE_COMMIT --head HEAD_COMMIT
cargo run --locked -p repo-devtools --profile ci -- ci select --full
```

Selection reads committed objects, not uncommitted edits, and runs no game checks.
Literal `include!`, `include_str!` and `include_bytes!` calls are tokenized as Rust,
including raw/escaped strings, raw identifiers, comments, whitespace and all three
delimiters.
Uncertain glob/path/manifest inputs select the full suite. `ci run skills|rust|policy`
reads `CI_SELECTION`. Build that controller with `--target-dir target/ci-controller`
as the workflow does; its Cargo children retain the ordinary target directory,
so Windows can rebuild the tested binary without replacing a running executable.
It requires matching checkout HEAD, streams ordered child logs
and stops on the first failure. Its final line is versioned JSON. `ci gate` reads
`CI_SELECTION` and `CI_NEEDS`; malformed/duplicate JSON and missing results fail.

Hosted classification runs real-Git docs-only, tools-only, game-only and shared-input
fixtures plus executable GitHub-output/final-gate regressions. These verify routing
and protocol behavior on a runner; observing job skips for a particular PR requires
that PR’s actual workflow result, not just a passing fixture.
Structural skill tests do not replace bounded native/model evaluations of changed
guidance; those remain separately recorded acceptance evidence.

`repo-devtools distribution check` asks Cargo for each library package's file list, rejects
escapes/symlinks, and stages those sources without games or tool packages. It builds an unrelated
consumer using empty, pure-algorithm, UI and native-networking feature selections,
checking the activated dependency graph for game/tool or networking leakage. Use
`--case empty|pure|ui|network` for a focused probe. Temporary sources are deleted;
artifacts reuse `target/`. This proves independent source consumption, not registry
publication, a complete game, visual quality or cross-machine networking. The native
probe does not open sockets or invoke Tailscale. CI runs all four cases on three OSes.

`distribution archives` repeats those cases using actual Cargo-produced library
archives. Its temporary staging adds sibling versions and uses `--exclude-lockfile`;
consumers patch unpublished siblings to inspected extracted files. These are explicit
artifact probes, with registry resolution and final library lockfiles still unverified.
`bundle check` runs for skill changes and affected tooling; `bundle verify-package`
checks the CLI archive when its package is selected. Narrative docs and isolated
game edits do not request the bundle or complete distribution checks. Bundle jobs
fetch full Git history to verify the preparation commit where available. After a
squash, byte/digest checks still enforce current committed inputs, while missing
historical commit verification is explicitly reported as unavailable.

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
