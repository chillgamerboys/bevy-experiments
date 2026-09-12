# Focused Gamekit workspace

Historical decision. Its flat-layout requirement is superseded by the
[repository organization refactor](repository-organization.md); the ownership and
single-workspace principles remain.

Accepted 2026-09-09. Recovery baseline:
`4add0a930ae76331d95ad747007d0cb4f4c13204` (merged PR #16).

Labyrinth is the primary game. Carterfight is retained as a contrasting UI adopter;
deckbuilder is a runnable UI/network regression example. Games retain composition,
rules and presentation ownership. Capabilities and the canonical seven-skill pack
are one initial release unit, pinned by tag/SHA. Future library distribution stays
private initially; extraction is not part of this change.

Retire Tactics, its phase notes, the legacy browser-packaging workflow and unused
tracked artwork instead of adding an archive directory. Tactics was a 10x10,
two-versus-two bump-capture prototype, not an HP-based tactics engine. Its final
local test run had ten passes and three failures (highlighting and turn transition).
Deleting those tests is retirement, not a claim that their failures were fixed.
Carterfight's seven rules tests passed immediately before its port.

## Implemented layout and retirement

The Bevy 0.19 workspace is promoted to the repository root, with Labyrinth as the
default package. Carterfight's backend, retained assets and tests live with its
ported composition root. Deckbuilder remains a compact regression adopter. There
is one lockfile; the legacy root package and nested workspace are gone.

Tactics source/tests/phase notes, the old browser launcher, superseded refactor
prose, six unused Carter poses and the unused fighter ZIP are removed. The four
runtime Carterfight inputs (font, ready sprite, cursor and WAV) are retained under
that game. No archive, compatibility path or replacement shared combat engine was
introduced. The execution plan is replaced by this record and current shared/game
documentation.

Shared UI mechanics no longer imply a skin. Explicit semantic skins, baseline
metrics/spacing, accessible labels, motion preferences and independent activation
readers support both the dark Labyrinth battle and Carterfight's pixel dialogue.
Labyrinth's battle and shell modules are split by presentation responsibility;
rules and encounter semantics are unchanged.

Deckbuilder now uses persisted, acknowledged admission with game protocol v3.
The obsolete admission authority and pure-session compatibility re-exports are
removed after migrating callers. Shutdown revokes authorization immediately, and
late traffic/removal from an old listener cannot mutate a replacement session.
Closing during admission also removes a just-persisted, not-yet-welcomed credential.

## Local cleanup

The two old build trees (approximately 6.4 GB and 55 GB), reviewed `.context`
contents (approximately 392 MB), skill Python caches and verified-empty obsolete
directories were deleted after the decision/evidence audit. A scratch launcher
symlink was unlinked without following it. Validation repopulates one root target
directory; the pre-build total is not a claim of net disk savings.

Tracked removals are recoverable from the baseline. Discarded untracked scratch is
not recoverable through Git; caches require rebuilding. No global toolchains/caches,
application profiles, credentials, Conductor history/metadata or linked workspaces
were removed. Clearing scratch does not reset agent or application conversation
history. `.context` is empty; new review output is ignored under `target/review/`.

## Implementation evidence — 2026-09-09

Local macOS checks passed after independent review fixes:

- Workspace all-feature CI-profile tests: 199 passing, including two doctests;
  two opt-in tests are excluded from the default run. Explicit documentation tests,
  strict all-target/all-feature Clippy, formatting and dependency policy also pass.
- The explicitly invoked Labyrinth four-process test kills and restarts a guest,
  reloads its credential, reconnects to the running host and finishes the fight.
  This is same-machine evidence, not a LAN/firewall claim.
- Minimal discovery/multiplayer graphs, the minimal App helper, and wasm builds of
  geometry/turns/session/UI/Labyrinth rules pass without importing game binaries.
- Four repository-policy tests and nine skill installer tests pass. The seven-skill,
  two-client validator proves structural parity, not behavioral agent selection.
  Generated manifest hashes now use canonical `/` paths; malformed/legacy separators
  fail explicitly without modifying the adopter.
- Labyrinth and Carterfight each have reviewed static frames at 1280x720,
  1920x1080 and 3840x2160, at Auto and 200% semantic scale. Additional menu/host/pause
  and intro/damage/outro frames cover their distinct presentation states. Small/high-
  scale layouts deliberately scroll secondary content. Layout/input tests remain
  separate from those rendered observations.
- All three default-feature CI-profile binaries survive bounded startup checks from
  both the root and their package directories. Carterfight captures verify loaded
  font/sprite/cursor assets; these startup checks do not prove audio playback.

The current root build output is approximately 26 GB after the cold rebuild, with
`.context` still empty. Review logs/captures are local ignored artifacts, not a
permanent source archive. The future extraction recipe has a disposable history-
filtering fixture check, not a performed extraction of this repository.

Pending acceptance: native pointer/keyboard/scroll/resize/modal/motion and Carterfight
audio review. Native automation cannot attach (`cgWindowNotFound`), so screenshots
and synthetic input do not close the interactive gate. Distinct-machine LAN and
Tailscale discovery/join/gameplay also remain unverified. The three-platform CI
configuration includes all games and now runs Python policy/skill checks on each
OS; a local macOS pass is not a claim that the new remote CI run has passed. Keep
the change in draft review until the outstanding required gates are resolved.

## Baseline evidence, not current acceptance

PR #16 recorded same-machine encrypted App and four-process guest-restart checks,
plus static review frames at three viewport sizes and two semantic scales. Those
old captures/logs are intentionally discarded in this reset. Cross-machine LAN and
Tailscale behavior and an interactive pointer/keyboard/resize/motion walk were not
verified; native window attachment failed. New UI changes need new evidence.

## Presentation decisions retained from scratch

Labyrinth preserves stable actor/status identity across snapshot updates. Status
inspection exposes potency, lifetime and timing without depending only on color or
hover. The timeline represents the current rolled initiative, never a prediction
of an unrolled next round. Detailed game rules stay in the game documentation.
