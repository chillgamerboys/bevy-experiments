# Historical verification milestones

These observations retain their original revision and evidence limits. They are
not current verification results.

# Labyrinth verification

## Rust GameSkills trial — named characters and formation preview

Driving Blow now deals 3 damage and attempts a push of up to two ranks. It stops
at the formation edge or before crossing an occupant wider than the remaining
distance. Lethal damage still suppresses the push. The shared effect resolver
reports attempted movement and limits to previews; the UI does not calculate a
second movement result. The content fingerprint is
`0a1fdad6fcc990599e90a545c1ab40e4d313b5de1ce51060f70d8cc5cb2142e9`;
multiplayer participants need matching builds.

The current company is Alden (Gatekeeper), Mara (Knifehand), Rowan (Scout),
Iris (Field Medic), and Ember (Lantern Wagon); Sera names a sixth hero when present.
Names are assigned from stable hero IDs within the encounter roster, independent
of class, rank, snapshot array order and life state. Monsters use their type names.
The battlefield, initiative, inspector, effects, forecasts and combat history share
these names. Hero classes remain secondary inspection information. Names do not
change actor IDs, ownership or the wire schema.

Select Driving Blow (Alden's third equipped ability), then a front enemy.
The after-action strip shows destination ranks, with gold markers for every moved
occupant. Against the initial Iron Brute, the Hauler shifts from 3–4 to 2–3 and the
Brute moves from 2 to 4. Against the initial Ash Brute, the push covers only one rank:
the remaining rank cannot cross the two-rank Hauler. Live sprites and hit areas stay
fixed until Confirm. Changing the turn, clearing selection, opening a menu, pausing,
or concealing necessary facts removes the preview.

The installed Rust CLI records the project-owned checks and their prerequisites:

```sh
gameskills run labyrinth-test labyrinth-lint rust-format repo-check
gameskills run labyrinth-movement-test
gameskills run labyrinth-render-movement labyrinth-render-movement-blocked labyrinth-render-movement-wide labyrinth-render-movement-large-text
gameskills evidence validate RUN_ID
```

`labyrinth-test` requires `rules-test`. Render commands reserve the shared GPU,
window and Cargo resources. The `movement` and `movement-blocked` capture routes
use real legal forecasts over a deterministic, undamaged Gatekeeper decision;
selection in the capture harness is authored, not native pointer evidence.

Pure regressions compare preview and committed effects for full/partial/blocked
pushes, whole large-unit movement and lethal suppression. Production UI tests cover
pointer/keyboard selection and confirmation, stable live anchors, whole-footprint
markers and disclosure/decision revocation at normal sizes and 200% scale. Inspect
the four rendered frames separately for legibility and clipping. Complete names
stay on one line, with HP or projected ranks on the next line. Compact identity
labels fit their columns at 11–18 logical pixels, independently of body-text scale;
full class/type detail remains available through inspection. Tests count actual
rendered glyph lines as well as checking bounds. Short windows with
enlarged text reserve a compact forecast lane before selection, so the preview
cannot overlap live names or shift the sprites. Regression coverage
includes repeated-class names, wire snapshot round trips and name-bearing log events.

The September 11 native walk used the rebuilt app, an isolated `names-trial`
profile and seed 42. Real enemy actions ran while the earlier heroes waited.
Alden selected Driving Blow with shortcut 3 and compared the initial Ash Brute
(one rank, Hauler limit) and Iron Brute (two ranks) using the pointer. Escape
cleared the preview without changing HP or formation; shortcut 3 and Space
recreated it on the focused Iron Brute. Tab reached Confirm and Enter committed:
Iron Brute fell from 20 to 17 HP and moved from rank 2 to 4, while the intact
Hauler shifted from ranks 3–4 to 2–3. The preview cleared for Ember's next decision.

The native window was resized, and the settings menu switched to 200% text.
Monster names stayed on one line. An Ember/Iris reposition preview also remained
readable and was cancelled; the menus returned to the game and Auto scale was
restored. Native screenshots and accessibility observations are saved under
`target/review/labyrinth-native-*.jpg` and `.txt`. The game is left open for owner
review. Human feedback on the damage/positioning tradeoff remains separate;
verified input and rules do not establish balance or enjoyment.

## Handoff follow-up — September 10, 2026

The follow-up to `e073da9` passes 113 application tests, 52 rules
tests, and the complete workspace all-feature suite (331 tests/doctests). The
separately enabled six-process abrupt guest-restart gate also passed. New coverage
includes encrypted request bursts, a noisy prefix followed by a quiet peer,
unauthorized requests, queue overflow followed by same-identity reconnect with the
watermark preserved, and an oversized encrypted Hello rejected before decoding.
Shared budget tests cover count/byte edges and independently served peers.

Presentation tests cover partial corpse damage, clearing with the before-pool HP
maximum, ordinary lethal damage, dying-hero death/rescue, hidden-health bystanders,
forecast bars and accessible labels, effective condition timing, and revocation of
open help when status disclosure is removed. Catalog prose changed the content
fingerprint to `53e0f0d6572c2408d126b0b82ce61a6a03af2f8d98398aca26105665281c622f`;
all multiplayer participants need matching builds.

The native local walk used the real reducer: damage and Bleed; Ash Brute death;
corpse Bleed at round end; a displayed `3 → 0 / 5` clear forecast followed by actual
removal/formation compaction; rescue of a dying Scout; subsequent death saves and
permanent death; and the wagon moving from ranks 5–6 to 4–5 while the Medic moved to
rank 6. The Scout corpse remained through round 5 after creation in round 3. The Mac
locked before the planned round-6 expiry and resize checks; these remain incomplete
as native interaction evidence. Pure lifecycle regressions cover expiry separately.

Authored capture routes `corpse-forecast` and `corpse-help` reproduce the two fixed
presentation paths. Normal-scale 1280×720 and 1920×1080 images are under
`target/review/followup-corpse-*.png`; they are presentation fixtures, not a gameplay
recording. At 1280×720 the actor forecast card uses its existing scroll viewport;
the full clear explanation is visible without scrolling at 1920×1080. Local Clippy,
formatting, dependency policy, library-only consumers, repository/skill tooling
checks passed. WebAssembly checks passed for hex, turns, session, UI and rules;
discovery/multiplayer builds without default features, the minimal test helper,
and standalone UI tests also passed. Cross-machine networking and remote CI
results were not verified in this run. Earlier milestone evidence follows.

## Footprint and corpse milestone baseline

The footprint/corpse milestone adds pure lifecycle tests, a five-App real UDP wagon
admission/action/fresh-client reconnect test, and native UI geometry/selection tests.
Existing six-player transport and abrupt-process-restart tests remain in place.
Capture routes `footprints` and `corpses` show the large units alive and as authored
corpse presentation fixtures. They are static evidence, not simulated death saves.

Final local macOS verification for this milestone: 51 rules tests and 105 application
tests pass, plus the explicitly enabled six-process guest-kill/restart test. The full
workspace all-feature suite (including doctests), strict Clippy, formatting, dependency
policy and repository ownership/link checks pass. The default formation regression
requires every enemy to have an in-range attack: Brutes 1–2, Hauler 3–4, Stalker/Archer
5–6. Five-App encrypted wagon admission/action/fresh-client reconnection is covered
separately from the explicit six-single-rank capacity fixture.

Rendered review covers the new living lineup and authored corpse layout at 1280×720,
and living lineup at 1920×1080. Native interaction checked keyboard ability selection,
focus traversal, pointer target selection and confirmation advancing to the next hero.
This is not a complete manual death-save playthrough or cross-machine multiplayer test.

## Six-player foundation review

Local macOS checks cover the six-seat reducer, repeated-class ownership, custom
equipped loadouts, real encrypted six-App sessions, and an explicit six-process
kill/restart followed by a completed fight. CI runs that process gate on all three
platforms; a local macOS pass is not a report of remote CI results.

## Stage-first UI review

The sprite/HUD pass retains twelve visible art hit regions at all six size/scale
combinations. Essential identities and current HP remain near actors; full names,
owners and effect definitions are inspectable. The battlefield does not scroll.
Long equipped-ability rows scroll horizontally; primary target legality and Confirm
remain outside that scroll area. Static frames cover combat, effect overflow,
inspection and the disconnected-player overlay.

Tests include explicit ability → target → Confirm with no early intent, native
window cursor/mouse-message hit testing (without assigning Interaction), modal
pointer blocking, drawer focus trapping/restoration, shortcut suppression, keyboard
scrolling to final content, and camera/DPI projection. These are complementary to
the existing authority, restart and replay tests, not replacements.

Local macOS workspace all-feature tests/doctests, strict Clippy, formatting,
dependency policy and repository boundary/link checks passed for this pass. The
six-process restart gate also passed on the final UI revision during pre-merge
verification. Real cross-machine routes remain unverified. Build output retains
the macOS linker `__eh_frame` size warning.

The native review build launches. The computer-use tool still cannot attach to this
unbundled executable (`Invalid app`); a fallback desktop walk was stopped when focus
changed. No completed interactive walk is claimed. A user pointer/keyboard/resizing
pass and cross-machine discovery/play remain manual gates; neither is established
by captures or same-machine tests.
