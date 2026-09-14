# Labyrinth

An original up-to-six-player cooperative positional-combat prototype. Battles are configurable on both sides; maze exploration is future work. It has HP, per-round initiative,
rank-constrained Skills, multi-rank creatures, bleed, death saves, rescue and corpses. There
is no stress, PvP, campaign, loot, or host migration.

## Play

From the repository root:

```sh
cargo run --locked -p labyrinth --profile ci -- --local
```

Local mode opens the battle setup lobby, controls the whole party and opens no game transport. The Prototype stock option has four ordinary heroes plus a two-rank Lantern Wagon. The default seed
is 42; use `--seed 91` to try a different reproducible fight. Plain
`cargo run --locked -p labyrinth --profile ci` opens the main menu; choose **Play with friends** for
hosting, discovery, direct joining or reconnection. No `--all-features` is needed.

The native launcher requests 1920×1080 logical dimensions with Auto UI scale.
Use `--window-size 1440x900` for a smaller desktop; the OS may constrain the actual
window and Retina framebuffer pixels differ from logical dimensions. `--help`
lists the launch options. See [setup and launch](../../docs/setup-and-launch.md)
for prerequisites, build profiles and GameSkills installation; none of its tests
or agent launch commands are required just to play.

Every hosted company has six participant slots, independent of its heroes. Each side has up to six formation spaces; a two-rank wagon uses two spaces but does not consume another player slot.
For six instances on one computer, build once, then run the resulting binary in
six terminals with distinct profiles:

```sh
cargo build --locked -p labyrinth --bin labyrinth --profile ci
./target/ci/labyrinth --profile host
./target/ci/labyrinth --profile guest-a
./target/ci/labyrinth --profile guest-b
./target/ci/labyrinth --profile guest-c
./target/ci/labyrinth --profile guest-d
./target/ci/labyrinth --profile guest-e
```

On Windows use `target\ci\labyrinth.exe`. Profiles isolate reconnect storage
and have an OS-held exclusive lock: accidentally launching two copies of one
profile fails rather than overwriting another guest's identity. `--data-dir PATH`
changes the application-data root; keep the same root/profile when restarting.
No admission secrets are accepted on the command line.

### Host and join

1. Host a company. For same-computer direct testing enter `127.0.0.1`; otherwise
   leave the advertised address empty to choose a local interface, or explicitly
   enter the reachable LAN/tailnet IP. The transport port defaults to UDP 7777.
2. Open **Lobby** in preparation and copy a **different** private BGN1 invitation for each guest. Invitations are
   single-use after acknowledged admission and initially expire after one hour.
   Reissue an invitation from an open lobby when necessary. These codes are bearer
   secrets; do not put them in screenshots, public chat, logs, or bug reports.
3. Guests paste their invitation into Join Direct. Alternatively, the host enables
   LAN discovery (or explicit development tailnet discovery) with an 8–64 character
   printable ASCII temporary passphrase (no leading/trailing spaces), and guests
   select its listing and enter it.
4. The host assigns places directly on the formation to give each participant zero,
   one or several characters. Guests can choose a character type in their assigned
   places and customize owned builds; the host controls enemy setup and movement.
   Unassigned places belong to the host. Guests begin as spectators.
   Only participants with assigned characters must connect and ready before Deploy.
   Empty reservations and spectators, including a host with no heroes, do not gate deployment. Changing
   battle setup or ownership clears readiness.

### Configure a battle

Preparation shows the party and enemies facing each other on one six-rank board.
Select an empty rank to browse character types, inspect their build and footprint,
then explicitly place the chosen type. Select an existing character to Customize,
Replace, Move or Remove it. Multi-rank creatures occupy their actual span. Replacing
keeps the character's identity/controller and resets its build and starting
conditions to the selected type. Moving keeps the build and takes the destination's
player assignment; removing leaves the other characters in their chosen places.

Gaps are allowed while constructing either side. **Deploy** requires occupied ranks
to be contiguous from the front; its explanation identifies gaps to repair. Smaller
test formations may leave unused ranks at the rear. Deployment never silently
reorders the lineup. The player strip shows who is preparing, ready or spectating;
co-op ownership controls sit beside the selected place. Local mode controls the
whole company without participant configuration.

In **Scenario**, load Prototype, Weapon Comparison, Cleave or Rescue/Status; each
option explains its test purpose. **Lobby** contains co-op connection/invitation
details. Select a character on either side and choose **Customize** for the same
unified editor.

The same character editor handles either team. **Parameters** opens first, followed
by **Equipment**, **Skills**, **Abilities** and **Moveset**. The character's actual
sprite, draft name, stats and equipment remain visible beside the scrolling content.
Parameters contains name, maximum HP, speed and rank size, with starting HP/statuses
in a separate Starting conditions group. Presets are chosen when creating a character.

Selecting a catalog entry only inspects it. Read its effects, ranks, uses,
prerequisites and proposed changes before choosing Equip/Unequip or Add/Remove.
Skills and Abilities separate personal selections from read-only **From equipment**
grants. Missing prerequisites leave personal selections inactive; restoring suitable
equipment reactivates them. Moveset shows eligible Skills with effective values,
source grants and passive upgrades. **Save & close** submits the complete draft and
waits for the host's acknowledgment. **Discard changes** reloads the saved character
in place; closing or switching a changed draft asks before discarding it.

Guests edit owned builds and select types in reserved places; the host controls
formation movement/removal, assignment and enemies. Starting HP can be blank for
full health or zero for a dying hero. An all-down draft is allowed, but at least one
hero must stand before deployment. Readability scales retain the same editor with a browser/detail route
when columns no longer fit.

In Scenario, set an explicit seed and use Save/Load with a local Scenario JSON path.
Save requires a deployable formation. Files contain battle configuration, not
construction gaps, credentials or participant identities. Loading restores compact
ranks; changing only the seed preserves current construction positions. Rematch returns
to the lobby with exactly the same configuration and seed. Saved JSON also exposes
controller policies and initial status source/duration for test harnesses; the UI
selects existing content rather than authoring new effect definitions.

Content lives in [`rules/content/catalog.toml`](rules/content/catalog.toml): stable
IDs define Skills, passive Abilities, weapons and actor presets. Add content using
existing typed effects there; new effect semantics require tested Rust behavior.
See [the content and scenario contracts](docs/architecture.md#content-builds-and-scenarios).


The temporary passphrase is **not an account password**. Do not reuse an important
password. Native Bevy's editable text currently displays typed characters; the form
warns about this and clears its owned buffers after submission. Passwords travel
only over the advertised certificate-pinned encrypted connection. The host keeps
an Argon2id verifier, not the plaintext passphrase. Application diagnostics redact
secret wrappers; this is not a claim that OS clipboard or networking-library buffers
can be scrubbed by the application.

LAN discovery requires a common multicast domain and OS local-network permission;
it does not cross Tailscale. Tailnet discovery uses an externally installed and
authenticated Tailscale CLI and UDP 7778, and is never enabled implicitly. Allow
UDP 7777 for gameplay and UDP 7778 for tailnet probes in host/peer firewall policy.
See [network diagnostics](../../gamekit/docs/multiplayer.md) for route and permission checks.

### Controls and combat

Pointer activation and Tab / Shift+Tab, Enter / Space work on native UI controls.
Select an Skill, inspect its source/target-rank requirements, select a target,
then Confirm. Unavailable Skills remain inspectable; confirmation explains why
the selected action cannot currently be submitted. Escape opens the Game menu
from combat and returns through its pages.

The battlefield uses replaceable 2D sprites with a minimal native overlay. H1–H6
and E1–E6 are stable actor labels, not ranks. Starred heroes are yours in co-op.
Hover/focus a character for its name, owner, rank, disclosed health and speed.
Click an initiative portrait to pin that character's card without changing the
selected target; it includes this round's speed, d8 roll, total and turn state.
Click a conditions badge for current effects and links to their definitions.
There are no separate Inspect or initiative-details menus. These cards do not
block combat confirmation; T enters keyboard reading and × closes the card.
Every granted move remains inspectable off-turn; only Confirm commits. Number keys 1–8 are shortcuts for the first eight moves; Tab and scrolling reach the complete list.

The combat log starts **hidden**. Its toolbar icon opens one compact, translucent,
non-blocking view of the complete **current encounter**, including after reconnect.
The wheel and Page Up/Down or Home/End browse older events. New events and arriving
pages preserve your reading position; **Latest** returns to the newest entry.
**×** hides the panel and retains your place; incoming events never reopen it.
Hover or focus a row to inspect its full text, including long names. The usual
two-second pin, keyboard reading, × dismissal and menu suspension apply.

History uses a complete encounter archive and bounded authenticated pages. Combat
snapshots still carry only the recent 80 events, while the log mounts at most 32
rows at once. Starting a new encounter resets history; past-game persistence is
outside this feature. History inspection and paging never execute combat actions.

The **Game menu** and its Settings page are local. They block your combat input,
but the encounter and networking continue. Escape backs out of menus; outside a
menu it opens Game immediately, preserving selection and temporarily hiding pins.
Leaving requires confirmation and explains whether it closes the hosted company,
releases a lobby seat, or leaves a reserved combat hero waiting for reconnection.
The command dock groups all resolved Skills and utility actions as glyph
controls, with automatic rows for larger movesets. Hover or keyboard-focus an Skill for its explanation; this never
changes the pending command. Descriptions float in the upper battlefield, outside
the command dock's layout. Their clicks do not select characters behind them.
Long descriptions scroll with the wheel or Page Up/Down and Home/End; the HP and
status strips and Confirm stay clear. The dock and numeric labels retain their
footprint when selecting Skills or targets. Initiative portraits open inspection without
retargeting. The paired six-rank diagrams face the same direction as the formations.

An Skill shows authored base power before targeting. Selecting a valid target
adds an immediate HP forecast on that character's health bar and in the dock;
The target's contextual card expands the immediate forecast; Skill and condition
cards explain base effects and their rules.
Forecasts do not advance combat or predict the next turn's damage. Bleed is shown
as conditional ticks, not guaranteed future damage. The prototype palette and
glyph strokes are game-owned `LabyrinthAppearance` tokens, not fixed Gamekit styling.
At large text sizes, long Skill rows scroll horizontally while the battlefield
and confirmation remain in view. Opening an effects badge reveals all effects;
for example `Ble2 / 3t+1` means Bleed potency 2, three bearer-turn boundaries left,
plus one other effect. The inspector gives the exact trigger and duration wording.

The presets are Gatekeeper, Knifehand, Scout, Field Medic and Lantern Wagon, with
repeated classes allowed within six spaces. The wagon has a weak scrap attack and
small limited heal; future supplies/navigation utility is not implemented yet.
Local and multiplayer defaults use each original hero once plus the wagon.
The prototype enemy lineup has a two-rank Ossuary Hauler and one each of Ash Brute,
Iron Brute, Wound Stalker and Hollow Archer in a **linear formation**,
not a hex grid. Starting order follows the scenario, independently of participant assignment.
The Brutes start at enemy ranks 1–2, the Hauler at 3–4, and Stalker/Archer at 5–6.
The Hauler attacks over the frontline; it does not block the Brutes' attack positions.
Move swaps adjacent whole combatants, never half a large actor. Both teams roll
effective Speed + d8 each round; the current
round's order stays fixed when actors move or speed changes.

Every actor freezes its **Moveset** at encounter start. **Skills** are active
commands; **Abilities** are passive effects or Skill upgrades. Characters and
weapons can grant either. Optional weapon-kind or exact-item requirements determine
whether a selected Skill enters Moveset and whether an Ability contributes. Personal
selections remain inactive when requirements are missing; restoring the equipment
reactivates them. Equipment-only grants cannot be added as personal selections.
Inspection shows all grant sources and applied upgrades without multiplying an
Ability granted through multiple sources.

The catalog includes six comparison weapons and five exact starter variants that
preserve the ten presets' existing actions. Resilient shortens finite negative
status durations by one on application or refresh, with a minimum one tick. Saved
remaining durations are not shortened again. Stats are provisional; inventory,
ammunition, reactions, progression trees and hotbar customization remain future work.

Bleed deals 2 damage at the affected actor's next three turn starts. Reapplying it
refreshes duration without stacking damage. Brace reduces direct damage by 2 until
the owner's next turn starts, but does not reduce bleed. Rescue recovers a dying
ally at 25% maximum HP, rounded up. Dying heroes roll a provisional d20 death save
at their initiative slot: below 10 adds a failure; three failures mean permanent
death. A nonzero hit while dying adds one failure. Success holds on without healing.

Player and monster corpses preserve all occupied ranks, have separate HP equal to
one quarter of living maximum HP (rounded up), and retain Bleed without refreshing
it. Corpse effects tick at round end. Remains can be attacked by either team and
expire after three full rounds, excluding creation: a round-2 corpse clears at
round-5 end. Destruction/expiry compacts the formation toward the center. Both sides
always retain six fixed rank spaces: smaller rosters and cleared corpses leave
empty back ranks without enlarging the characters, menus or controls. Two-rank
actors retain their full footprint. Healing/rescue cannot revive
corpses. All heroes down loses; all living enemies dead wins even with corpses left.
The host can return to the lobby for a new test encounter; this is not campaign revival.
See [formation and death contracts](docs/rules.md).

### Disconnect and restart

If a required controller disappears, the host retains their living/dying heroes and pauses combat once transport loss is observed. Spectator disconnects do not pause; permanently dead characters no longer require their owner. Dying characters retain ownership for rescue. Restart with the same profile and choose Reconnect.
The client persists an offered rotating credential **before** acknowledging it;
the host does not authorize commands before that acknowledgement. Private invitation
retries and persisted-credential retries recover the same pending hero after lost
offers/acknowledgements. If an initial **password** admission loses its offer before
the guest can persist a credential, its anonymous pending reservation expires after
15 seconds; wait for expiry before retrying the password to avoid a temporary extra
reservation. Established reconnection does not use the password.

Leaving the lobby explicitly releases the reservation and returns owned heroes to
the host. Leaving during combat keeps the participant reserved for reconnect.
The host can open Game menu → Party management → Pause and edit assignments,
reassign surviving characters to connected participants, then explicitly resume.
Opening or returning from a menu never resumes a paused encounter. This changes control only: HP,
uses, initiative and status clocks do not advance. Stale commands are rejected even
if the same hero was assigned away and back. A rules fault cannot be cleared this way.
Host process restart ends the session; guest credentials cannot recover a lost world.

The current Labyrinth protocol is **v8**, adding bounded authenticated encounter-history pages to canonical Skills, passive
Abilities, Moveset and equipment prerequisites,
sparse lobby positions, rank reservations and separate setup/assignment revisions. It is
incompatible with earlier builds; catalog and scenario schemas are version2.
Old save formats are rejected without overwriting the originals. All participants need matching builds/catalogs
and a new hosted company; existing credentials are not silently repurposed.

## Engineering boundaries

`rules/` is a pure Rust package: no Bevy, network, window, or filesystem dependency.
It owns rules and content. `src/session.rs` owns players, readiness, replay policy,
disconnect/fault suspension, and encounter lifecycle. `src/network/` composes opt-in Gamekit capabilities.
`src/ui/` projects snapshots and sends typed intents; it never mutates authority.
`src/presentation.rs` owns viewer facts and immediate forecast formatting. Its
unknown-information fixtures conceal exact HP, effects and inspection details;
identity, allegiance, rank and standing/downed state remain public. Current network
snapshots still include all facts: real reveal rules require server-side recipient
filtering, not merely hiding UI text. Labyrinth always installs shared Gamekit
tooltips. The capability remains optional for other game composition roots and
does not own combat layout, appearance, or disclosure policy.

### Ability inspection

The automatic action rail shows glyphs and numbered shortcuts, not permanent Skill
descriptions. Select an Skill, select a target, then Confirm. Formation numbers
remain plain ranks. Emphasized footprints mark usable source positions and legal
targets; a stronger selected-target highlight and actor emphasis distinguish the
current selection. Exact source/target ranks are listed in Skill tooltips;
HP forecast segments remain attached to the affected actor.

Hover an Skill or status to see its card immediately. Leaving before 2 seconds
hides it immediately; continuous hover locks it with an accent border. Locked
cards support related terms and stay open while hovering or clicking other
characters and Skills. Each pinned card has a **×** control that closes that
card and its linked descendants; closing the root clears the chain. Outside clicks
preserve pins. **Escape** opens the Game menu directly, including while inspecting
a pin or selecting an action, and returns through its separate Settings and Party
management pages. Menus hide all tooltips and restore still-valid pins on return.
Close the current card before inspecting another source. **T** explicitly
opens keyboard inspection of the focused source; merely retaining clicked-button
focus never reopens a preview. Game-menu and combat-log toggles have no tooltips.
**K** toggles the equipped skillbook,
which uses the same disclosed Skill content. Neither inspection nor a skillbook
link spends a turn. The shared timing and key bindings can be changed independently
of Labyrinth's rules and palette.

See [architecture and extension contracts](docs/architecture.md)
and [verification](docs/testing.md). The future endless maze should
be another game-owned model/orchestrator, not a reason to put expedition rules in
Gamekit or to replace this combat kernel.

Battle UI and character customization landed in PRs #42–45. Current behavior is documented above and in [architecture](docs/architecture.md).
Earlier foundation and separate acceptance context: [weapons, character builds and configurable battles](docs/plans/weapons-and-battle-setup.md).
