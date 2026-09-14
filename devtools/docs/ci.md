# CI selection and verification


[The workflow](../../.github/workflows/gamekit.yml) resolves project verification
policy, then selects affected packages and suites through
[`repo-devtools ci`](../src/ci/mod.rs). The branch policy maps `dev` to Development and `main` to Testing.
The repository currently receives PRs on `main`; creating and adopting `dev` is
[pending rollout](../../docs/setup-and-launch.md#current-rollout-state). Both use macOS for classification, policy, skills, Rust and
the aggregate gate. Windows/Linux are selected only at Release under this project's
policy. These are project choices, not GameSkills defaults for other adopters.

The standalone `gameskills verification resolve --base BRANCH [--level LEVEL]`
command emits versioned JSON. The CI adapter consumes that output, preserving its
receiving branch, level, platforms, display, digest and selection reasons. The
resolver owns rigor; this adapter owns affected packages and command/journey mappings.
Cargo's `--profile ci` only selects the build profile. A lower explicit rigor cannot
weaken the receiving branch's requirement, and Release selection does not publish.

Narrative docs avoid game/runtime jobs. Skill instructions select structural checks
and affected runtime tests. Game changes select positive owner suites and relevant
compile/Clippy checks. Shared libraries also select reverse consumers and applicable
minimal-feature/browser-core or distribution checks. Testing evaluates the combined
batch's affected components and interactions. Unknown, CI or uncertain dependency
inputs broaden component investigation/coverage within the resolved level; they do
not silently select every OS, display or end-to-end journey.

Suites distinguish rules/content, session/persistence, UI behavior, transport/admission,
process recovery and display compatibility. Logic fixes do not automatically run an
agent UI tour. UI changes use affected behavior and normal 1080p Auto checks; retained
compatibility cases are separately selected. End-to-end coverage follows the changed
boundary: a game edit alone does not require the six-process restart test. Test
selection lists matching tests first and rejects zero matches. Changed/unmapped tests
need classification or bounded broader owner coverage so new regressions cannot be
silently excluded.

The selector reads committed base and tested-tree manifests, including normal,
development, build and target-specific local dependencies. PR checks compare the
event's base commit with the tested GitHub merge tree; pushes compare before/after
commits. Renames/deletions retain old owners. Static Rust file inclusions count as
build inputs alongside the file's normal owner. Literal `include!`, `include_str!`
and `include_bytes!` calls are tokenized, including raw/escaped strings, comments and
all three delimiters. Unsupported dynamic inputs broaden affected selection.

To inspect selection locally, supply committed IDs and the resolved policy file:

```sh
gameskills verification resolve --base dev > .context/verification-policy.json
cargo run --locked -p repo-devtools --profile ci -- ci select --base BASE_COMMIT --head HEAD_COMMIT --policy .context/verification-policy.json
```

`ci select --full` means full affected scope at the supplied rigor, not Release.
Selection runs no game checks and reads committed objects rather than uncommitted
edits. `ci suite NAME` runs a named positive suite. `ci run skills|rust|policy` reads
`CI_SELECTION`, requires matching checkout HEAD, streams ordered child logs and stops
on failure. Build the controller with `--target-dir target/ci-controller` as the
workflow does; its Cargo children use the ordinary target directory. This permits
Windows to rebuild the tested binary without replacing a running executable.

The always-run policy and final gate bootstrap the checked-out repository tool with
locked dependencies and the pinned toolchain. Bootstrap failure fails the job; no
older binary substitutes. `ci gate` reads `CI_SELECTION` and `CI_NEEDS`; malformed
or duplicate JSON, failed/cancelled/missing jobs and unexpectedly skipped selected
jobs fail. No Python setup/interpreter is needed. The workflow does not alter branch
protection. Dispatch takes an explicit `level`; it is not an implicit full release.

CI reports automated results separately from milestone manual acceptance. A Testing
batch affecting game behavior needs the developer's actual candidate-bound sanity
response in delivery/audit. Docs/tooling without game effects do not need gameplay
sanity. A successful CI job or agent walkthrough cannot supply that human response.

Routing fixtures exercise docs, logic, UI, networking, shared inputs, receiving
branches and gate failures. Their success establishes controller behavior; actual
workflow observations establish the deployed Development/Testing routes. Release
routing fixtures are not an observed cross-platform release. Structural skill checks
likewise do not replace bounded model/native observations for changed guidance.

`repo-devtools distribution check` asks Cargo for each library package's file list, rejects
escapes/symlinks, and stages those sources without games or tool packages. It builds an unrelated
consumer using empty, pure-algorithm, UI and native-networking feature selections,
checking the activated dependency graph for game/tool or networking leakage. Use
`--case empty|pure|ui|network` for a focused probe. Temporary sources are deleted;
artifacts reuse `target/`. This proves independent source consumption, not registry
publication, a complete game, visual quality or cross-machine networking. The native
probe does not open sockets or invoke Tailscale. Selected policy controls its OS
coverage; Development/Testing do not gain Linux/Windows from a distribution check.

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

For changed UI presentation or interaction, use 1920×1080 Auto and the evidence
layers needed by the affected claim. Logic-only fixes need no agent frames or UI
walk. Compatibility sizes/scales are retained for relevant defects or explicit
Release commitments. Keep captures under `target/review/` through review, then retain
a concise revision/command/result record. Stop after applicable checks pass unless
new changes, failures or unresolved concerns justify more verification.

## Superseded branch runs

Pull-request concurrency is per PR. Push concurrency is per receiving branch, so
newer dev/main pushes cancel superseded push runs while the latest combined source
is checked. Explicit workflow dispatches keep independent run identities. A cancelled
run is never counted as passing evidence for the revision it did not finish.
