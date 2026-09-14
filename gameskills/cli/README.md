# GameSkills Rust CLI

The unpublished `0.1.0-dev.4` candidate installs immutable GameSkills instructions,
constructs native Codex/Claude invocations, coordinates durable work queues and
executes configured command graphs with verifiable evidence. Its 13 core skills
use `plan` as the default entrypoint; eleven optional skills live in six specialist
packages. The internal Rust modules are not a stable public API.

```sh
cargo install --locked --path gameskills/cli
# In the adopter's Git root:
gameskills catalog
gameskills setup
gameskills setup --apply
gameskills status
gameskills native codex --verify
gameskills native codex --verify-project
```

Initial setup uses the instruction snapshot embedded in the executable. On an
existing project, bare `setup --apply` also selects that embedded snapshot; use an
explicit matching `--bundle` to hydrate or retain another recorded pin. See the
[checkout procedure](../docs/installation.md#this-checkout-and-existing-pins). It needs
no source checkout, network bundle fetch, Cargo or interpreter at runtime. Git is
required for repository operations. The source installation needs Rust 1.97.1;
prebuilt candidates are target-specific. There is no Bevy, GameKit or repository-tool
runtime dependency. Configuration and instructions remain TOML, JSON and Markdown.

`setup` reports a proposal until `--apply` is supplied. Core-only installation is
the default. `--packages gameskills gameskills-ui` selects an explicit combination.
Use `setup --bundle PATH --apply` for a compatible immutable update or rollback,
and `setup --recover` to recover an interrupted config/lock/registration transaction. Local
instructions, overlays and previous bundles retain their ownership. Updates that
conflict with newer local edits fail with paths to inspect.

Selected Codex clients receive persistent project `.codex/config.toml` registration
as part of setup. The generated marketplace source is absolute and machine-local;
keep those entries out of shared commits while preserving existing owner settings.
`.gameskills/native-registration.json` records owned values for safe updates and
cleanup. Disabled or changed owned settings are conflicts, not repair permission.
Neither setup nor native commands change global Codex configuration or project trust.

Repair an older staged installation without repinning:

```sh
gameskills native codex --register
gameskills native codex --register --apply
gameskills native codex --verify-project
# Recover only an interrupted targeted registration transaction:
gameskills native codex --register --recover
```

Registration is serialized with setup but can repair the existing pin during an
unfinished queue; it leaves the GameSkills config and lock unchanged. Changed project
settings invalidate command evidence even when Git ignores the native config or
ownership record. Copied absolute paths are reported as
outdated. `--verify-project` uses plain `codex app-server --stdio`, never generated
enable flags or automatic trust elevation. It reports observed catalog/cache results
separately from registration and any existing host session's activation.

An interrupted queue write may leave a `.queue-<pid>-<serial>.tmp` file. Setup and
recovery preserve recognized ordinary files under the queue lock without treating
their incomplete contents as queue state. Unknown entries and unsafe files still
block the operation, as do unfinished queues and active command runs.

`config validate --file PATH` checks standalone TOML without setup or Git. Bare
`config` requires installation readiness. Global `--root` defaults to the current
directory. `catalog`, help and version do not require an installation. `native`
prints literal client arguments; `--launch` starts the selected client and
`native codex --verify` checks bounded app-server discovery without a model turn.
Neither structural validation nor discovery proves model behavior.

`status` scopes its top-level `ok` to valid installed files/configuration and exposes
`native_clients`: Codex has `registration`, `project_registration_ready`, mismatched
settings or conflict detail, and separate discovery/session/trust states. Discovery
is unobserved until a verifier runs; status does not relabel an earlier result as
current. Missing/drifted registration is an actionable defect. Claude persistent
registration remains unsupported; its explicit session launch is still available.
After registration, open a new Codex/Conductor session or restart the host as needed.

Plans retain authored schema 1; Rust installation locks, queues and evidence use
schema 2 with an explicit runtime identity. Finish active Python queues with their
original runtime before adoption. Existing records and bundles remain untouched;
old observations cannot become validated Rust passes. Rerun checks with Rust to
obtain new evidence. `legacy import` preserves the old seven-skill installation
and project overlays while proposing the new installation.

Queue operations, command supervision and native Codex verification currently require
POSIX support. Windows receives explicit unsupported diagnostics for those operations;
portable configuration, catalog, packaging and installation checks have separate
coverage. Authenticated native-client behavior and game playtests are separately
recorded acceptance evidence, not inferred from an operating-system test matrix.

```sh
cargo test --locked -p gameskills-cli --profile ci
cargo clippy --locked -p gameskills-cli --all-targets --profile ci -- -D warnings
cargo package --locked -p gameskills-cli
```

Targeted runner integration tests need a freshly built subprocess probe:

```sh
cargo build --locked -p gameskills-cli --profile ci --example runner_probe
cargo test --locked -p gameskills-cli --profile ci --test runner
```

Selecting `--test runner` omits Cargo's usual example build. An older probe can
contain a previous CLI runtime and reject the current instruction bundle. The
repository's `rigor-test` command builds this prerequisite explicitly.

Successful commands return JSON; diagnostic failures have `error.code` and
`error.message` with exit 2. A failed execution observation exits 1. Help/version
are text. Native launch preserves its process status. Diagnostic wording and the
internal supervision protocol are not public compatibility promises.

`bundle/` contains a generated manifest and deterministic archive from canonical
`gameskills/plugins/` instructions. Maintainers regenerate with `repo-devtools bundle prepare`
and verify with `bundle check`; Cargo packages carry those exact bytes. No build
script downloads content and no runtime path points back into this workspace.
Compiled examples are test probes, not additional installed programs.

Publication remains disabled. The workspace declares `MIT OR Apache-2.0`; the
actual license/notice files, registry names and release ownership still require
the planned public-release audit. See [distribution](../../docs/distribution.md) and
[adopter guidance](../docs/installation.md).


## Verification policy

Verification is opt-in and independent of creative involvement and Cargo build
profiles. Existing adopters without `[verification]` keep their current checks.
A project can configure its delivery base separately from the current feature branch:

```toml
[project]
delivery_base = "dev"

[verification]
default_level = "development"
manual_sanity = "milestone"

[verification.branches]
dev = "development"
main = "testing"

[verification.display]
width = 1920
height = 1080
scale = "auto"

[verification.levels.development]
platforms = ["macos"]
[verification.levels.testing]
platforms = ["macos"]
[verification.levels.release]
platforms = ["macos", "windows", "linux"]
```

All three level tables are required, with nonempty, unique platform lists. Higher
levels retain the platforms required below them. Branch names are short Git branch
names, not refs or revision expressions. The optional display has positive dimensions
and currently supports `scale = "auto"`. `manual_sanity` accepts `milestone` or
`never` (the default); it describes gate timing, not a human acceptance result.

```sh
gameskills verification resolve
gameskills verification resolve --base main
gameskills verification resolve --base dev --level release
```

Resolution reads configuration without setup, Git, writes or an agent session. It
returns schema 1 JSON with `configured`, `receiving_branch`, `branch_required_level`,
`level`, `platforms`, `display`, `manual_sanity`, `policy_digest` and `reasons`.
The digest binds the complete normalized policy and default delivery base. The base
defaults to `project.delivery_base`, or legacy `main` when absent. An unmapped base
uses `default_level` and reports that reason. An explicit level below the receiving
branch's requirement (or the default for an unmapped base) fails; requesting Release does not authorize publication.

Without `[verification]`, `configured` is false, level/display are null, platforms
are empty and an explicit `--level` fails rather than inventing requirements. The
project adapter still owns affected packages, suites and journeys: this resolver
selects policy, not test commands or coverage. Unknown impact can broaden affected
component selection without escalating platforms or displays to Release.

These fields require CLI 0.1.0-dev.4 or newer and a compatible instruction bundle. Update
through the supported setup path; do not reinterpret historical records or edit
installed cache files. See [verification and evidence](../plugins/gameskills/references/verification.md#rigor-and-affected-scope)
and [milestone sanity](../plugins/gameskills/references/delivery.md#milestone-sanity).

### Persisting task scope and observations

```sh
gameskills delivery start damage-fix --goal "Correct damage rounding" --base dev --scope rules --gameplay --check rules-test
gameskills run rules-test --base dev --scope rules
gameskills delivery scope damage-fix --scope rules --gameplay
```

`start` records resolved policy and caller-classified scope; `--gameplay` identifies
game effects for milestone sanity. `scope` explicitly binds or updates that selection,
preserves prior selections and the original task/base, and retains the previous level
when `--level` is omitted. Repeated `--check` options replace the required command
list while preserving its prior value; omitting them keeps the existing checks. Supplying the complete scope/gameplay classification is the
caller's responsibility. Neither command executes checks or claims human acceptance.

`run` accepts `--base`, `--level`, and repeated `--scope` labels. Its record includes
the policy and scope alongside the selected command graph. Evidence must cover the
task's scope labels, policy and receiving base at the same or a stronger level.
Policy or scope changes require reassessment; old records are not relabeled. For
commands with selected Git inputs, the runner additionally binds the actual receiving
base, preferring `refs/remotes/origin/BASE` and falling back to `refs/heads/BASE`.
Fetch the intended branch before running checks when neither exists.

A configured gameplay milestone requires an explicit observation with
`delivery check TASK --manual-observation FILE`. Paths are relative to `--root`.
The file contains schema 1 JSON with `task_id`, `source_head`, `verification_digest`
(the task's `verification.selection_digest`), `result` (`passed`), `observer`,
`journey`, and `evidence_reference` pointing to the developer's actual response.
Its task, candidate and selected scope must match. The result labels this as
caller-supplied evidence: the CLI does not authenticate the person or perform play.
Missing/stale/failed observations leave milestone acceptance incomplete; ordinary
Development tasks and tooling-only milestones need no gameplay observation.

## Tracker observations

Required tracking defaults to connected host MCP when no `tracking.observer` argv
is configured. Set `tracking.mode = "mcp"` explicitly for that choice. The invoking
agent supplies a fresh issue snapshot with `delivery check TASK
--tracker-observation FILE`; core checks its task/source/bindings and a live GitHub
backlink. The [delivery contract](../plugins/gameskills/references/delivery.md#tracking-observations)
defines the schema, 300-second freshness limit and caller-supplied evidence boundary.
No standalone tracker or separate API key is required. Existing argv-only config
retains command mode; `mode = "command"` makes that choice explicit. Conflicting
modes and attempts to substitute MCP evidence for command mode are rejected.

This additive configuration/flag belongs to the current development candidate;
older executables reject it. Rebuild or update the executable before adoption.

## Documentation discovery

`gameskills docs resolve --path PATH` reads optional `[docs]` and target docs
mappings from the adopter root; repeat `--path` for mixed work. It works before
setup, returns root/component indexes and diagnostics, and does not read all linked
pages. See [mapping behavior](../docs/architecture.md#documentation-discovery).

## Declared Git inputs for command evidence

Commands conservatively fingerprint all refs by default. When a command's Git
inputs are known, declare exact full refs to prevent unrelated worktree commits
from invalidating its evidence:

```toml
[commands.rules-test]
argv = ["cargo", "test", "-p", "my-rules"]
git_refs = ["refs/remotes/origin/main"]

[commands.review-diff]
argv = ["git", "diff", "origin/main...HEAD"]
git_refs = "all"
```

The runner unions selected commands and prerequisites; one omitted or `"all"`
policy keeps the entire graph conservative. An explicit `[]` selects no additional
refs. HEAD, symbolic branch identity, worktree/index contents, configuration,
executables and environment remain required inputs in every mode. Named refs must
exist; names, object IDs and symbolic targets are fingerprinted exactly. Declare
all refs actually consumed, including the review base. No argv inference or general
hermeticity is promised. See the complete [evidence contract](../plugins/gameskills/references/verification.md).

This additive field belongs to the current development candidate. Earlier binaries
may ignore it and continue hashing all refs. Updating the runtime or policy does
not upgrade existing evidence: preserve old records and run fresh checks.
