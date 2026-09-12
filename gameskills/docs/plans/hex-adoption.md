# Port Vila adoption pilot

Status: deferred
Owner: GameSkills/adopter integration. Resume after internal docs and framework work
and explicit selection of an adopter task. No installation is performed by this plan.

## Purpose and ownership

Use the existing [Bevy Hex Game](https://github.com/chillgamerboys/bevy-hex-game)
as a real test of incremental GameKit and GameSkills adoption. Labyrinth and
Deckbuilder remain the primary development consumers. This pilot should validate
installation, coexistence, useful UI integration and later updates in a project
that already has its own architecture, workflows and contributors.

The local checkout is the `port-vila` workspace. The user owns this GameKit
repository, but Bevy Hex Game has other contributors. Propose adopter changes
through focused PRs for their review, targeting its current integration branch
(`dev` at inspection time). Planning here does not authorize merging adopter PRs
or changing their shared branches. Refresh contributor work and repository
instructions before proposing the actual integration.

## Inspection baseline

Bevy Hex Game was inspected at
`bb556963632de933b44fb75b1d306aca79258cef`, with a clean `dev` checkout.
GameKit was inspected at reviewed commit
`89628a0095b2076cdf1ab2cc54fc38cbbd21bbc8`, whose source tree matches the merged
`b75f7ea199ce345306421f6c65b35ba5b464e8e0` revision. This is a historical baseline,
not the candidate to install automatically when the pilot resumes.

- Both workspaces declare Bevy 0.19. Feature resolution and actual compilation
  together remain to be checked.
- The current seven-skill install audit reported 29 generated-file additions and
  no file conflicts. No apply step ran. This establishes filesystem coexistence
  only; skill selection and instruction compatibility remain unverified.
- Port Vila already has local skills, headless UI tests, scripted image-target
  walks and a development casebook. Preserve those established contracts.
- `hex_ui` renders immutable view models and emits typed intentions. Its manifest
  dependency ceiling is enforced by a test as well as documented. Adopting a
  GameKit UI dependency needs a reviewed update to that precise boundary.
- Both UI foundations register Bevy keyboard navigation. Inspect focus,
  activation and scaling ownership before installing the complete GameKit UI
  plugin alongside the existing one.

Reference entry points at the inspected Bevy Hex Game revision:
[working agreements](https://github.com/chillgamerboys/bevy-hex-game/blob/bb556963632de933b44fb75b1d306aca79258cef/AGENTS.md),
[UI contract](https://github.com/chillgamerboys/bevy-hex-game/blob/bb556963632de933b44fb75b1d306aca79258cef/docs/systems/ui.md),
[Activity renderer](https://github.com/chillgamerboys/bevy-hex-game/blob/bb556963632de933b44fb75b1d306aca79258cef/crates/hex_ui/src/combat_log.rs)
and [dependency check](https://github.com/chillgamerboys/bevy-hex-game/blob/bb556963632de933b44fb75b1d306aca79258cef/tools/test_test_scope.py).

## Entry conditions

1. The internal [skills-first sequence](../architecture.md) has produced
   working, evaluated workflows and reconciled documentation.
2. A reproducible GameKit/GameSkills candidate has immutable source and package
   identities, compatible Bevy features, install/update instructions and recovery
   guidance. Do not depend on a neighboring developer checkout's path.
3. Refresh the adopter's branch, open PRs, UI priorities, skill inventory,
   dependency constraints and contributor ownership. The inspection above will
   be stale by then.
4. Agree on one useful UI outcome with the adopter's reviewers. The Activity
   feed below is a candidate, not an approved feature change.

## Proposed adoption sequence

### 1. Audit and introduce the skills

Run the candidate's install audit on the refreshed adopter. Explain each selected
skill's role and how it coexists with local guidance. Preserve existing skills,
project settings and generated-file ownership. Keep project-specific commands,
test selection, target branches and domain constraints in the supported local
configuration/reference mechanism.

Propose the installation through a reviewable PR. Verify discovery in the
supported agent clients and perform a real development task. Test overlapping
triggers and resumed work, not only file names. Record the actual installed
revision and distinguish it from canonical source.

### 2. Integrate one UI capability

The initial candidate is Activity feed scrolling: follow new entries while at
the latest content, preserve the reader's position while reading history, and
offer an explicit return to the latest entries. GameKit currently provides
`UiFeedScroll` and `GameUiFeedPlugin`; verify their minimal composition contract
before choosing the final adapter.

Keep event content, filtering, disclosure, styling and game state ownership in
Bevy Hex Game. Account for bounded history, tab changes, rebuilding rows,
hidden/reopened surfaces and Compact layout's shared scroll ownership. Define
the content revision semantics rather than using visible row count as a proxy.

Review the exact dependency-ceiling extension and prove that the UI feature does
not introduce networking or game dependencies. Check for competing focus,
scrolling and scaling systems. If the library needs a narrower public seam,
land and verify that change with Labyrinth and Deckbuilder first, then update
the adopter candidate pin.

Contextual help/tooltips are a possible later slice, selected from actual UI
needs. Whole-HUD replacement, game-rule migration and multiplayer consolidation
are outside this pilot.

### 3. Verify with the adopter's existing tools

Use the current project-selected automated checks and typed UI observations.
Use its noninteractive visual-walk path for static presentation review. Arrange
a named live interaction review through its normal process; do not launch or
focus a visible game merely because automation is unavailable.

Record what each check proves, its source revision and remaining limits.
Exercise pointer/keyboard input, focus, scrolling, resizing and state re-entry
for the affected surface. Test gameplay or disclosure claims at their typed
owner rather than inferring them from rendered text.

Submit the integration PR for contributor review, respond to findings, and
verify the landed `dev` revision when it is merged through that project's
process. Work in the shared checkout must remain coordinated with its ongoing
development; use an isolated candidate checkout when implementing the pilot.

### 4. Exercise maintenance and return the lessons

Perform a subsequent pinned update with an adopter-specific customization
present. Verify that updates preserve local ownership, surface conflicts and
can recover from an interrupted or rejected change using the supported workflow.

Record friction with enough context to reproduce it. Classify the correction as
GameKit API, GameSkills behavior, installation tooling, documentation or
adopter-specific code. Add the reusable cases to internal verification and
retain the reviewed adopter PRs as evidence.

## Completion and non-claims

Completion requires reviewed installation and integration, an actually consumed
candidate artifact, a verified update, and evidence that the existing game's
behavior and local workflows still work. A successful source-only build or
conflict-free skill audit does not complete this pilot.

A delayed or declined adopter PR leaves external adoption unverified; it does
not prevent independent internal work or a private Labyrinth playtest build.
General release claims should state which adoption and platform cases were
actually exercised.
