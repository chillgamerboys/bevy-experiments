# GameSkills architecture

The executable and immutable instruction bundle are separate identities. The core
package supplies focused development workflows; optional plugins add domain guidance.
Gamekit is optional to an adopter. The [catalog](catalog.md) owns skill boundaries.

## Runtime ownership

- `cli/src/config.rs`: project configuration structure and defaults.
- `cli/src/agents.rs`: opted-in routing against caller-observed host model capabilities; no model launch.
- `cli/src/usage.rs`: task/thread counter checkpoints, imported intervals and cost reports; no transcript disclosure.
- `cli/src/verification.rs`: deterministic project rigor and receiving-branch policy resolution.
- `cli/src/docs.rs`: read-only documentation discovery, independent of installation.
- `cli/src/installation/`: bundle verification, atomic setup, native adapters and migration.
- `cli/src/installation/registration.rs`: project-scoped Codex settings ownership and same-pin repair.
- `cli/src/workflow/`: revision-guarded work queues; queues do not launch agents.
- `cli/src/runner/`: configured commands, supervision, resource locks and evidence.
- `cli/src/delivery.rs`: solo intent and observed PR/tracker delivery.
- `plugins/`: canonical skill bodies and package-local references.
- `linear/`: optional provider helper; no core dependency.

These paths are relative to the GameSkills source directory. The modules support
the executable; they are not a promised stable public Rust library interface.

## Decisions

Task scope, execution strategy and execution authorization are separate. Recover
applicable existing authorization across handoffs; a worker limit describes capacity
and does not by itself grant permission. Durable delivery records carry the endpoint
for solo and coordinated work alike.

Project configuration owns commands, targets and local workflow choices. Skills
retain triggers, necessary judgment and completion criteria; current project docs
own local facts. Resolve docs from the adopter root and APIs from its actual locked
dependencies, not this development checkout or sibling plugin-cache paths.

Native packages remain self-contained. Portable references carry reusable craft;
project indexes route to local architecture, development and troubleshooting. A
pointer states when to read a source, what to establish, and the next action.

Setup preserves project-owned instructions and overlays. Immutable bundles, recorded
pins and command evidence are not authoring locations. Keep historical evidence
unchanged; source/config/ref/environment changes may invalidate a current claim.
The validator names changed input categories without exposing their secret values.

No helper forces an agent to invoke a skill or finish its task. Native discovery,
model behavior, command success, PR publication and human acceptance are different
observations. [Contributing](contributing.md) describes their verification.

Codex setup includes project registration, not just immutable file staging. Owned
marketplace/plugin values are tracked separately from unrelated owner TOML. Setup
recovers config, lock and registration together; targeted registration recovery
changes only native settings and their ownership record. Both refuse intervening
local edits and unsafe paths. Same-pin registration takes the setup lock but does
not require queues/runs to be inactive; normal evidence identities still observe
the changed project files, including ignored Codex config, its ownership record and
an interrupted registration journal. Installation/pin changes retain the stronger exclusions.

Ordinary-project discovery has a separate native probe with no injected enable
flags. Project trust, host overrides and existing-session reload remain explicit
boundaries; no global configuration or trust edit is implicit in setup. Claude's
session-scoped launcher does not establish persistent Claude registration.

Tracking uses the host's connected MCP by default when no command observer is
configured. Core validates a fresh, task/source-bound normalized issue snapshot
and the live GitHub backlink; it records the snapshot and digest as caller-supplied
evidence, not an authenticated MCP invocation. The agent owns the actual connector
call and raw evidence. This keeps provider credentials out of core and avoids
requiring an extra helper for a connection the host already supplies. Existing
observer argv configurations retain command behavior; mixed modes fail explicitly.

## Verification policy and acceptance

Optional `[verification]` separates verification depth from creative involvement.
The core resolver owns the normalized policy and identity; project adapters own
changed-input analysis and command/journey selection. CI exchanges versioned JSON
with the resolver, keeping repository tooling independent of CLI implementation and
avoiding separate platform defaults in workflow YAML. See the
[CLI configuration contract](../cli/README.md#verification-policy).

A receiving-branch requirement is a floor for explicit level selection. Unknown
impact broadens affected coverage within that level; it does not silently become
Release. Existing configurations and historical evidence keep their original
meaning. `project.delivery_base` is explicit and defaults to `main` for adopters
that omit it; changing the current feature branch is not part of resolution.

Policy selection, command execution, agent observations and developer sanity are
separate evidence. Milestone acceptance for affected game behavior needs the actual
developer response and its candidate/journey applicability; caller-authored records
cannot independently authenticate a human. Ordinary development has no milestone
gate. Documentation/tooling-only work without game effects needs no gameplay tour.

## Command ref identity

All refs remain the command default because trusted commands can read arbitrary
Git state. A per-command `git_refs` list explicitly narrows that dependency to exact
full refs; selected commands and prerequisites contribute a union. Any default or
`"all"` policy preserves all-ref behavior for the graph. HEAD and its symbolic branch
remain unconditional, as do source/index/configuration/executable/environment
identities. A declared ref's deletion or movement invalidates evidence; unrelated
branch activity need not invalidate a scoped graph. Missing named refs fail before
execution. Delivery observations and nested submodule identities stay conservative.

The policy is additive configuration, not a reinterpretation of historical evidence.
Runtime/normalization/configuration changes require a new observation and preserve
old records. The [CLI contract](../cli/README.md#declared-git-inputs-for-command-evidence)
explains selection and development-version compatibility.

## Documentation discovery

Optional `[docs]` and `[targets.NAME.docs]` tables accept `index` and `plans`, each
relative to the repository root. Target mappings require a target `path`. Defaults
look for `docs/README.md`, then `README.md`, at the root and the selected target.
A plans location may be absent until a substantial plan exists.

The most specific enclosing target wins by path component. Declared index parent
folders and plan folders also route changes to docs kept outside the source tree.
Equal target paths or equally specific competing docs owners are errors. A mapped
root README identifies that file, not every file in the repository. Multiple input
paths return a deduplicated set of indexes while retaining the root owner.

Explicit indexes must be files, plans locations directories when present, and
existing symlink ancestors must stay inside the repository. Cross-owner links inside
the repository are allowed. Missing conventional indexes are diagnostics rather than
invented files. The command returns locations only and performs no network calls.

Older configuration remains supported. Mappings require CLI 0.1.0-dev.3 or newer;
an older CLI rejects the unknown field. Upgrade explicitly before adding mappings.

## Efficient delivery and usage

`agents.routing` is opt-in and validated independently of installation. The
project owns model IDs and bounded escalation; a supplied host capability snapshot
must support the selected model and effort. Resolution records requested settings,
not proof of native execution. Native observations belong to task usage receipts.

Usage is a separate ledger rather than a token field fabricated by the command
runner. Preserve per-thread attempt intervals and missing counters; sum deltas once
across coordinator and workers. Optional caller-supplied pricing yields an estimate
with source/date, never inferred billing. Task telemetry has no merge authority.
See the [command and data contract](agent-workflow.md).
