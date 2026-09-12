# Documentation and skill guidance refactor

Status: active planning; implementation has not started.
Owner: shared repository work, tracked with [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows).
Inspection baseline: `b1a14cfc34c1f443bd6d369e266a7ed813d83d25`.
Related work: [GameSkills reliability](../../gameskills/docs/decisions/workflow-reliability-and-linear.md), draft PR #38.

## Outcome and accepted choices

Make current documentation explain the system that exists, how to develop it,
and how to diagnose problems. Make skills use that documentation deliberately,
with short instructions and portable references instead of competing manuals.

The user has settled these choices:

- Documentation follows ownership: shared repository, Gamekit, GameSkills,
  internal developer tools, and each consuming game.
- Lasting rationale belongs in a **Decisions** section of the relevant README,
  architecture page, or topic guide. No separate decision archive is required.
- Substantial active plans contain proposed changes, choices, implementation steps,
  and outstanding acceptance. Completed plans are removed after useful conclusions
  reach current documentation; Git and PR history supply the historical record.
- Review document content and skill prose together. Skills should find project
  concepts, avoid duplicated facts, and contribute to maintaining the docs.
- Define a shared default structure with explicit mappings for existing adopters.
  Installing GameSkills does not require another repository to adopt this layout.
- Linear deletion and the live Hex deletion pilot are deferred. Mandatory ticket
  backups are removed; no backup directory or deletion credential gates this work.

This turn delivers a plan. The implementation described below requires the next
implementation instruction. It does not authorize gameplay changes, a repository
split, edits to linked workspaces, merging, a public release, or parallel agents.
The MCP-first everyday tracking correction remains a separate open item in the
reliability work; this docs refactor must not claim that runtime fix was completed.

## Current problems to address

Inspection found 31 documents in the five main docs trees, plus product/package
READMEs and 24 skill entrypoints with packaged references. File count is not a
success metric. Important problems are mixed authority and lifecycle:

- `decisions/` contains active plans, deferred proposals, current contracts, and
  completed reports. `history/` repeats current rules and carries unresolved work.
- `development.md` sometimes describes using a product and sometimes maintaining
  it. Installation, CLI reference, and contributor instructions need clear owners.
- Root architecture currently asks for two plausible consumers before sharing a
  capability; the skill architecture reference says a second consumer is evidence,
  not a universal prerequisite. Resolve this deliberately, not by moving both texts.
- `update-docs` still mandates decision records. Several references and reports
  still describe the superseded mandatory-export/live-deletion acceptance rules.
- `devtools/src/support.rs::local_target` verifies local destinations but strips
  fragments. Existing link checks do not establish that a heading anchor exists.
- `gameskills/cli/src/config.rs` rejects unknown root fields. Documentation mapping
  is not an existing config capability; it requires an explicit compatible update.

## Target structure and authority

Keep topic files shallow. Create an index only when there are several pages to
navigate; do not scaffold empty documentation categories for small games.

| Owner | Current documentation after refactor |
|---|---|
| Repository | Root README; `docs/README.md`, `architecture.md`, `development.md`, `testing.md`, `distribution.md`; substantial shared plans under `docs/plans/` |
| Gamekit | README; `gamekit/docs/README.md`, `architecture.md`, existing `ui.md` and `multiplayer.md`; crate APIs in Rustdoc and focused examples |
| GameSkills | README; `gameskills/docs/README.md`, `installation.md`, `workflow.md`, `architecture.md`, `catalog.md`, `contributing.md`, `troubleshooting.md`; its active plans under `gameskills/docs/plans/` |
| Internal tools | `devtools/README.md` for commands and `devtools/docs/ci.md` for current CI mechanics/rationale |
| Labyrinth | README; architecture, rules, disclosure, and testing under its existing docs directory |
| Other games | Existing READMEs remain sufficient until a real topic needs a separate page |

These filenames are implementation defaults, not a demand for one file per topic.
Combine short related material when a separate page would add navigation without
helping the reader. A README is a starting point, not a second full manual.

One authoritative location owns each fact:

- Rustdoc, source and tests establish public API behavior. Guides explain how to
  use it and where extension seams are; they do not copy exhaustive API inventories.
- Project docs own local architecture, commands, conventions and troubleshooting.
  A Decisions section gives concise rationale for current constraints.
- Skills own the action: trigger, required context, procedure and completion claim.
- Packaged references own reusable Bevy/GameSkills craft. They must travel with
  their plugin and cannot require this development checkout's human docs.
- Plans own intended change and remaining work, never an alternative current API.

A docs/code disagreement is a finding to investigate. Documentation of an intended
constraint may expose a code defect; observed code behavior does not automatically
invalidate that constraint. Resolve against accepted intent, source and tests.

## Document disposition

The following covers all 31 documents in the inspected docs trees. Keep a temporary
paragraph-level inventory during implementation for mixed documents; do not create
another permanent migration ledger. Every unresolved item needs a destination.
Paths in this table are repository-relative.

| Existing document | Disposition and destination |
|---|---|
| `docs/README.md` | Keep as the shared navigation entry; distinguish current guides from active work. |
| `docs/architecture.md` | Keep workspace ownership and Decisions; move detailed capability contracts to Gamekit architecture and link them. |
| `docs/development.md` | Keep repository contributor setup, adding games and artifact locations. |
| `docs/testing.md` | Keep shared verification policy/entrypoints; link internal CI implementation and owner-specific test guides. |
| `docs/extraction.md` | Split current artifact/distribution contracts into `docs/distribution.md` and outstanding release/split work into `docs/plans/distribution.md`. Do not imply a published release. |
| `docs/gamekit-consolidation.md` | Distill current boundaries into Gamekit architecture; keep unimplemented extraction/balance work in `gamekit/docs/plans/capability-development.md`. |
| `docs/decisions/repository-organization.md` | Fold current names/layout/ownership into README and architecture; retire the completed execution plan and old path tables. |
| `docs/decisions/0001-focused-workspace.md` | Preserve still-current constraints and useful troubleshooting in their owners; retire the baseline implementation report. |
| `docs/history/handoff.md` | Verify remaining findings against current source and follow-up; move any residual work into its owner plan, then retire. |
| `docs/history/handoff-followup-plan.md` | Preserve current security/UI/forecast lessons and outstanding verification, if any; retire completed task narration. |
| `gamekit/docs/ui.md` | Keep integration contract, link API owners, reconcile duplicated skill guidance. |
| `gamekit/docs/multiplayer.md` | Keep operations/diagnostics, link Gamekit architecture; distinguish generic mechanics from game acceptance routes. |
| `gameskills/docs/README.md` | Replace historical entrypoints with task-oriented navigation for adopters and maintainers. |
| `gameskills/docs/development.md` | Split installation, daily workflow, maintainer work and troubleshooting into the named current guides. |
| `gameskills/docs/smoke-test.md` | Merge its short package-verification instructions into contributing guidance; retire the redundant page. |
| `gameskills/docs/decisions/gameskills-catalog.md` | Rewrite as current `gameskills/docs/catalog.md`; retain useful workflow explanation once, in workflow guidance. |
| `gameskills/docs/decisions/gameskills-framework.md` | Distill current purpose/architecture/Decisions; move genuinely unfinished work to `gameskills/docs/plans/framework-followups.md`; delete the staged historical proposal. |
| `gameskills/docs/decisions/gameskills-ci-scope.md` | Current CI mechanics/rationale go to devtools CI docs; relevant trial requirements go to GameSkills contributing guidance or active follow-up work; retire old routing proposal. |
| `gameskills/docs/decisions/port-vila-adoption.md` | Move to `gameskills/docs/plans/hex-adoption.md`, explicitly deferred; retain material acceptance and refresh stale inspection assumptions before use. |
| `gameskills/docs/decisions/workflow-reliability-and-linear.md` | Reconcile accepted scope and move to `gameskills/docs/plans/workflow-reliability.md`; keep active until its actual remaining work is resolved. |
| `gameskills/docs/decisions/workflow-audit.md` | Fold reusable failure lessons/limitations into workflow, troubleshooting and contributing guidance; retain needed source-bound evidence with the active reliability plan, then retire the separate report. |
| `gameskills/docs/history/gameskills-refinement-results.md` | Distill useful workflow/UI lessons and unresolved acceptance; retire chronological results. |
| `gameskills/docs/history/gameskills-rust-adoption.md` | Current compatibility/adoption limits go to installation/contributing docs; unresolved trials go to follow-ups; retire migration record. |
| `gameskills/docs/history/gameskills-rust-cli.md` | Current runtime architecture goes to architecture/CLI docs; unresolved release or runtime work gets an active owner; retire the completed migration plan. |
| `devtools/docs/ci.md` | Keep authoritative CI implementation guidance and incorporate still-relevant rationale. |
| `games/labyrinth/docs/architecture.md` | Keep current structure/extension contracts; link rules and disclosure rather than duplicate them. |
| `games/labyrinth/docs/decisions/footprints-and-death.md` | Rewrite implemented rules as `games/labyrinth/docs/rules.md`, with relevant Decisions; separate provisional/future content from current rules. |
| `games/labyrinth/docs/disclosure.md` | Keep current disclosure contract and its actual network-secrecy limitation. |
| `games/labyrinth/docs/forecasts.md` | Merge verification guidance into testing and behavioral explanation into disclosure; retire the small duplicate page. |
| `games/labyrinth/docs/testing.md` | Keep current checks/routes and incorporate forecast verification; historical pass counts are not current acceptance. |
| `games/labyrinth/docs/history/verification-milestones.md` | Preserve repeatable test routes and unresolved gaps; retire milestone narration. |

Review all entrypoint READMEs, including root, Gamekit/facade, GameSkills/CLI/Linear,
devtools and games, for stale destinations and duplicate authority. Retain the
Labyrinth asset README as asset-local guidance. Do not remove compatibility data,
frozen legacy source, provenance, generated instruction bundles, adopter overlays,
old installed pins or machine execution evidence as “historical docs.”

## Project documentation discovery

Prefer one index per owner over a second machine-maintained catalog of every
page. Introduce the following small optional mapping in `gameskills.toml` during
implementation; these fields and command are proposed, not available today:

```toml
[docs]
index = "docs/README.md"
plans = "docs/plans"

[targets.labyrinth.docs]
index = "games/labyrinth/README.md"
plans = "games/labyrinth/docs/plans"

[targets.gamekit.docs]
index = "gamekit/docs/README.md"
plans = "gamekit/docs/plans"
```

Add equivalent documentation target entries for GameSkills and devtools here;
reuse existing target paths instead of creating a competing ownership map.
All configured paths are relative to the repository root, including target docs.
The index points to architecture, development, testing and troubleshooting topics
that exist; several roles may share a page or section.

Proposed read-only command: `gameskills docs resolve [--path PATH]`.

- Return the root entrypoint plus the most specific enclosing target's index and
  plan location, with origin (configured or conventional) and diagnostics.
- Match path components, not string prefixes: `games/labyrinth-other` must not
  match `games/labyrinth`. Existing/missing file paths can be resolved lexically;
  configured index files must exist. Equal target-path mappings are an error;
  nested targets use the longest enclosing path and retain root context.
- For mixed changes, resolve each affected owner and deduplicate entrypoints;
  do not silently choose one game's concepts for shared work.
- Without mappings, discover `docs/README.md`, then `README.md`, at the root and
  resolved target. Missing conventional docs are reported as missing, not fabricated
  and not a blanket blocker for unrelated work. Explicit broken mappings fail.
- `plans` is an optional location, not a requirement that an empty directory exist.
  Reject absolute/out-of-repository mappings; detect symlinks escaping ownership.
  Index links may refer to another owner inside the repository intentionally.
- Do not recursively load every linked document. Skills select relevant sections
  from the index and task; source, Rustdoc and configuration remain direct inputs.
- A file change under a declared docs location must also resolve its mapped owner,
  even when an adopter stores that owner's docs outside its source target path.
  Ambiguous mapped documentation ownership is a diagnostic, not a guessed owner.

Adopters with existing documentation names opt into mappings without moving files.
Do not silently edit adopter config, global client settings or installed plugins.
Old config without `[docs]` remains valid. An old CLI rejects the new field, so
version the new capability, declare an appropriate instruction CLI compatibility
range, and explicitly upgrade the candidate before adopting mappings here. Preserve
old locks and run records; never rewrite evidence to make it match new inputs.

## Skill prose and references

Review all 24 entrypoints and their reachable references; change only content that
has an identified ownership, duplication, staleness or routing problem. Use a
short temporary matrix: claim, current copies, authoritative destination, and
skills that need to find it. Do not optimize for an arbitrary word count.

| Skill family | Required outcome |
|---|---|
| `setup` and shared project-context reference | Explain discovery/defaults/mappings and how to report missing docs without pretending they were read. |
| `grill` and `plan` | Read relevant current concepts first; research contradictions, record proposed constraint changes in the active plan, distinguish current guidance from proposals. |
| `debug`, `test`, `playtest`, `review` | Load relevant architecture, testing or troubleshooting sections; retain task-specific diagnostic and evidence judgment. |
| `update-docs` | Own current-doc reconciliation, Decisions sections, link repair, and completed-plan retirement. Remove the requirement for separate decision records. |
| `create-pr`, `audit-pr`, `merge-pr`, `release`, `dispatch` | Carry the same documentation obligations through delivery/resume; do not create parallel documentation pipelines or claim completion from links alone. |
| Optional UI, rules, multiplayer and maintainer skills | Keep portable craft in packaged references; consult adopter-owned rules, architecture and acceptance details only when relevant. |
| Optional Linear and Bevy contribution skills | Review for stale local assumptions without broadening scope; deletion stays deferred and upstream-specific constraints remain scoped to upstream work. |

A pointer must say when to read the target, what to establish, and what action
follows. Essential triggers, authorization boundaries, failure handling and
completion criteria stay in the skill body. Avoid both bare link collections and
chains of tiny references that require loading most of the catalog.

Packaged reusable references use package-local paths. Project docs are resolved
from the adopter context, never `../../../../docs` from an installed skill cache.
Gamekit API guidance resolves the adopter's actual dependency source/version.
Links to another optional package require supported discovery and a useful missing-
package response; core works without Gamekit, Linear or optional specialist plugins.

Resolve the extraction-rule discrepancy by preserving the supported general
principle: demonstrated need and an independently understandable contract justify
extraction; a second consumer is evidence, not a mandatory numeric gate. Record
any deliberately stronger project-specific constraint in that project's docs;
do not silently turn it into a universal GameSkills rule.

## Plan lifecycle and enforcement

Substantial plans use a title, a readable `Status: active` or `Status: deferred`,
owner/scope, accepted choices, steps and acceptance. Routine notes do not require a
plan file. Deferred plans state why they are deferred and when to reconsider them;
keep only details that will still help then. No new roadmap or archive is needed
merely to replace a deleted report.

During delivery, update current docs, preserve remaining work, fix links and remove
completed plans. A merged implementation with unresolved required acceptance is
not automatically a completed plan. For multi-PR work, keep the plan until the
whole outcome is settled. If closure can only be observed after landing, use a
small follow-up cleanup; do not require a ceremonial close-out PR for every task.

Deletion is an ordinary Git change; do not rewrite history. A plan added and
removed inside a squashed PR will not survive main's file history, so useful
rationale must reach the final docs/PR description. Do not introduce an export or
archival prerequisite to preserve obsolete narration. Repair current links;
external historical links can point to an appropriate retained commit when needed.

Implement enforcement in layers:

1. **Portable config/discovery:** CLI validates docs mappings and resolves context.
   It does not depend on repo-devtools, a network service or Linear credentials.
2. **Project structural checks:** extend existing repo-devtools Markdown checks for
   this repo's local paths and anchors, mapped indexes and active-plan conventions.
   Configure a `docs-check` command through the existing GameSkills command runner.
   Other adopters can use their own documentation checker under the same workflow.
3. **Semantic review:** reviewers compare changed behavior, docs and skill prose;
   confirm remaining work has an owner before retirement. Structural success does
   not prove prose accuracy, that an agent read a source, or behavioral usefulness.

Do not introduce a new shared Rust crate or duplicate the repository Markdown
validator inside the CLI merely for this work. Reuse the existing parser/checker
for repository and packaged-reference validation, with boundary-specific policies.
The CLI's new responsibility is document discovery, not a second general linter.

Anchor validation needs a documented supported Markdown subset and meaningful
fixtures: same-page/cross-file anchors, duplicate headings, inline formatting,
Unicode/punctuation, percent-encoded paths, reference-style links, and fenced code
examples. Support explicit HTML anchors if used. Ignore external URLs for the
local check; do not add unreliable network availability as a CI gate. Scope this
check to current owned docs/skills; preserve frozen fixture validation as-is.
Do not claim complete GitHub Markdown compatibility without establishing it.

The repository check can flag completed plans left in active plan directories and
retired current `docs/history/` / `docs/decisions/` paths after migration. It cannot
infer from a timestamp or PR state that a plan is complete, and must never delete
files automatically. Directory conventions apply to declared owner docs, not
arbitrary fixture paths or another adopter's pre-existing layout.

## Implementation sequence and review boundaries

1. **Resolve content and map destinations.** Inventory paragraphs in mixed docs,
   locate current code/commands, and assign unfinished work. Finalize the modest
   mapping/diagnostic contract before implementing it. This plan is the starting map.
2. **Add discovery and structural verification.** Extend CLI config/command dispatch
   and tests; extend repository Markdown checking and scoped CI tests. Keep old
   adopters valid. Add the documentation conventions to current development docs.
3. **Rewrite current docs and retire old material.** Work by owner. Move content and
   repair all inbound links in the same change; reconcile counts, commands, statuses
   and rationale rather than preserving stale paragraphs under new filenames.
4. **Refactor skill prose against those owners.** Update discovery, relevant lifecycle
   and specialist skills, remove duplicated local facts, then verify actual routing
   in clean adopter fixtures. No new documentation plugin or parallel skill pipeline.
5. **Package and deliberately adopt.** Regenerate the immutable instruction bundle
   from a committed canonical revision; build actual Cargo archives and test them
   outside this source tree. Upgrade this repository's CLI/pin before enabling its
   mappings. Do not modify linked repositories or installed caches as source.
6. **Review and deliver.** Reconcile active plans, README/index links, PR description
   and HEX-98. Report command, package, discovery and model-behavior evidence
   separately. Carry unresolved reliability work forward; neither deletion nor a
   public release is needed. Retire this plan only after the agreed outcome lands.

Use logical commits within the existing branch/PR unless a later user instruction
changes the integration shape. Do not create a ticket per document. The current
PR remains a draft while implementation/acceptance is incomplete; merging remains
separately authorized. This planning commit is not implementation of the sequence.

## Verification and acceptance

| Claim | Evidence required |
|---|---|
| Content has one current owner | Review every disposition above and all inbound README/skill links; all unresolved findings retain an active owner. |
| Adopter discovery works | CLI tests for default/custom layouts, README-only projects, nested/mixed targets, externally located owner docs within the repo, missing mappings, ambiguous roots and path/symlink boundaries. |
| Old adoption still works | Existing config/installation tests; core-only fixture with no docs mapping, no Gamekit and no Linear credentials. |
| References are portable | Actual packaged instructions/CLI in isolated consumers with conventional and custom docs layouts; no development-checkout paths. |
| Links and conventions are enforced | Repository/skill validators plus anchor/path/plan-lifecycle regressions, including frozen-fixture exclusions. |
| Skills remain useful | Review triggers and retained procedural judgment; realistic planning, debugging and completed-plan cleanup exercises that require different owner docs. Include a stale-doc contradiction and a moved heading. |
| Delivery remains honest | Existing delivery/resume regression suites where configuration or routing changes affect them; source-bound command records, observed PR and declared evaluation gaps. |
| Verification stays proportional | Docs-only changes select docs checks; executable Markdown/skill changes select relevant skill/runtime checks; no gameplay or full game build solely for prose movement. |

Run the existing repository, skill, legacy, bundle and relevant Rust/config/native
checks for changed inputs. Parser/runtime changes require focused tests and Clippy;
package changes require actual archive consumers. Do not rerun gameplay or visual
reviews without a changed game behavior or unresolved game-specific claim.

Forward model exercises should test behavior, not mere link presence. Start with
solo exercises under this repository's existing authorization; independent agent
or host trials need applicable authorization and availability. Never relabel an
unrun independent evaluation as passed or add evaluators implicitly. Any unavailable
claim stays explicit at handoff rather than blocking unrelated documentation work.

## Remaining design checks during implementation

No additional product preference is needed to begin this plan. Resolve these
bounded technical checks from code and tests before their dependent edits:

- Confirm the smallest supported Markdown anchor implementation and document its
  limits; reuse a suitable parser only if current parsing cannot meet the fixtures.
- Verify how target docs mappings survive setup/config normalization and appear
  in evidence identity. Relevant docs changes must invalidate claims that depend
  on them; do not broaden/narrow fingerprints merely to keep a run green.
- Verify install compatibility for the new config capability before bumping bundle
  metadata; older pins must continue to explain their incompatibility explicitly.
- Refresh every supposedly completed historical item before deletion. In particular,
  release readiness, network-disclosure limits, external adoption and model trials
  must not disappear because their old report was removed.
