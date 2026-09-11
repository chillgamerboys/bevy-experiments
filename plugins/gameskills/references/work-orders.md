# Durable plans and dispatch work orders

This is the version-one contract shared by `plan`, `dispatch`, and dispatch's
injection mode. Use a compact plan for a solo task; create a queue when work needs
durable coordination. The runtime validates repository observations and records
state. The invoking agent still investigates, obtains any required authorization,
starts workers through the host's actual mechanism, collects checks, reviews work,
and performs authorized integration. The runtime never launches workers, executes
acceptance checks, invokes other skills, merges Git branches, or contacts GitHub.

## Commands

Resolve the compatible Rust `gameskills` executable and the installed core package. `ROOT` is the
actual repository checkout root that owns this queue, not an arbitrary child
folder. These helpers require project setup/configuration.

```sh
<gameskills-executable> --root <repo> plan validate --file PLAN.json
<gameskills-executable> --root <repo> queue create --file PLAN.json
<gameskills-executable> --root <repo> queue status wave-one
<gameskills-executable> --root <repo> queue inject wave-one --file ORDER.json --expected-revision 1
<gameskills-executable> --root <repo> queue start wave-one ui-copy --worktree <worker-checkout> --expected-revision 2
<gameskills-executable> --root <repo> queue report wave-one ui-copy --file REPORT.json --expected-revision 3
<gameskills-executable> --root <repo> queue block wave-one ui-copy --file BLOCK.json --expected-revision 4
<gameskills-executable> --root <repo> queue resume wave-one ui-copy --worktree <worker-checkout> --expected-revision 5
<gameskills-executable> --root <repo> queue integrated wave-one ui-copy --file INTEGRATION.json --expected-revision 7
```

The examples show command syntax, not an executable sequence: resumed work must
be reported again before integration. Read `queue status` for the current
revision. Every mutation after creation requires that revision. A stale revision,
invalid transition, dependency error, or collision leaves the queue JSON
unchanged. Reload status and reassess before retrying; do not silently replay a
stale decision. `dispatch --inject` is the skill mode; `queue inject` is its helper.

`plan validate` does not create a queue or change project settings. All commands
return JSON through the entry script. Validation errors produce a failing command;
a recorded report does not imply that its claimed checks passed.

## Plan schema

A plan is a JSON object. `schema_version`, `id`, `goal`, `delivery_target`,
`repository`, `versions`, `packages`, `decisions`, and `orders` are required.
`creative_level` defaults to the configured level. An order inherits the plan's
creative level and packages unless it explicitly chooses a subset of packages
and its own bounded creative level.

```json
{
  "schema_version": 1,
  "id": "wave-one",
  "goal": "Clarify the established end-turn interaction",
  "delivery_target": "pr",
  "creative_level": 2,
  "repository": {
    "root": "/absolute/path/to/repo",
    "base_commit": "REPLACE_WITH_FULL_COMMIT_ID",
    "source_commit": "REPLACE_WITH_CURRENT_HEAD"
  },
  "versions": {"bevy": "project-declared version", "gamekit": null},
  "packages": ["gameskills", "gameskills-ui"],
  "decisions": ["Preserve turn legality and keyboard bindings"],
  "orders": [
    {
      "id": "ui-copy",
      "goal": "Explain why end turn is unavailable",
      "artifact": "Committed UI correction and regression evidence",
      "target": "game crate: turn UI",
      "creative_scope": "Improve wording and feedback within the accepted interaction",
      "owner": {"kind": "agent", "name": "ui-worker"},
      "execution": {"role": "implementation", "effort": "session-default"},
      "decisions": ["Use the existing disabled-button explanation path"],
      "investigation": ["Inspected the turn UI at the recorded source revision"],
      "references": ["Project UI guidance and the relevant source symbols"],
      "files": [{"path": "src/turn_ui.rs"}],
      "resources": ["native-game-window"],
      "dispatch_blockers": [],
      "merge_blockers": [],
      "acceptance": ["The disabled action explains the actual blocking condition"],
      "expected_evidence": ["Focused check result and actual native interaction observation"],
      "coordination": ["Return shared rule changes to the coordinator"]
    }
  ]
}
```

Use actual full lowercase Git commit object IDs; the placeholders above deliberately
are not valid input. `source_commit` must be the coordinator's current HEAD when
validating or creating a plan and must descend from `base_commit`. Git supplies
the canonical checkout root and common repository directory; an explicit mismatched
`common_dir` is rejected. Versions are project declarations, not a runtime claim
that Bevy or GameKit compatibility has been tested. Set `gamekit` to `null` when
unused. Packages must already be selected in project configuration.

IDs are 1–64 ASCII letters, digits, underscores, or hyphens, beginning with a
letter or digit. They cannot contain path separators or traversal components.
Delivery targets are `design`, `implementation`, `pr`, `merge`, and `release`.
Creative levels are integers 1–4. Neither field grants permission to start agents,
publish, or merge. Execution role/effort fields communicate the intended host
settings; they do not configure a model or prove its actual identity. Additional
agreed budget metadata may be retained in the order, but this helper does not
measure or enforce token or time budgets.

Every order requires all the fields shown. String-list fields can be empty when
inapplicable; `acceptance` and `expected_evidence` must have at least one entry.
`files` can be empty for work that owns no repository files. Preserve concrete
source locations, observations, accepted decisions, affected contracts and
coordination conditions in the corresponding fields; a vague checklist is not a
substitute for investigation. Extra contextual fields are preserved. Runtime
state belongs in the queue entry, never in these work-order fields.

## Ownership, resources, and dependencies

`owner.kind` is `agent` or `human`; `owner.name` identifies the contributor.
Human-owned work is not automatically reassigned. Human streams participate in
ownership, resources, and integration dependencies without consuming worker slots.
A pending human stream reserves its resources unless it explicitly depends on the
stream being considered. Blocking a human stream retains its reservation.

Ownership paths are literal repository-relative POSIX paths. Glob patterns, dot
segments, Git/runtime metadata paths, and paths through symbolic links are
rejected. A trailing slash owns a directory subtree. A file entry without `lines`
owns the whole file. An optional inclusive range allows separate declared regions:

```json
{"path": "src/turn_ui.rs", "lines": [40, 85]}
```

Overlapping ranges, whole-file overlap, and directory/file overlap collide.
Unsequenced file collisions are rejected at planning and injection. The helper
compares declared boundaries; it does not parse Rust symbols or prove that edits
stayed within them. Coordinate line shifts, changed ownership and shared manifests,
lockfiles, registries, fixtures, or assets with the integration owner before work
continues. A change to an existing order needs replanning and affected-owner
coordination; injection cannot rewrite an existing order.

Resources are exact string identities such as `native-game-window`, `gpu`,
`audio-device`, or `tcp:24000`. They are exclusive while a worker is running.
Plans may queue agent orders sharing a resource; the next start waits until the
resource is released. Use explicit sequencing or one owner when artifacts from
resource use also create file conflicts. Host CPU/memory limits and worker launch
permission remain coordinator responsibilities.

Both dependency arrays contain order IDs from the same queue. Unknown IDs,
self-dependencies, duplicate IDs, and cycles across either kind are rejected.

- `dispatch_blockers` gate starting or resuming. The prerequisite must have
  returned work (`reported` or `integrated`), and the consumer checkout must
  contain the prerequisite's returned Git HEAD. A reported prerequisite is
  rechecked for changed HEAD or uncommitted changes. A returned report is useful
  work, not an automatic review/acceptance decision. This permits dependent work
  before integration when the actual prerequisite commits are available.
- `merge_blockers` gate recording integration. Each must already be `integrated`.
  They do not prevent otherwise independent workers from starting.

File overlap is allowed only when a transitive dispatch dependency establishes
sequencing. A merge blocker alone does not make simultaneous file ownership safe.
A reported human predecessor permits its dispatch-dependent consumer to start
before integration, provided the returned source is unchanged and the consumer
contains its returned HEAD. Pending, running and blocked human streams retain
their existing reservations; unrelated work cannot borrow a reported human
stream's territory or resources without the established dispatch dependency.
A blocked agent with outstanding checks cannot release a slot: first collect or
stop the actual checks, then explicitly report `checks_running: false`.

Agent starts require `dispatch.enabled = true`. The active worker cap defaults to
5 and must be 1–5. A project can choose fewer slots. Human starts do not consume
slots. Every start and resume rechecks that the order's packages remain selected
in current project configuration; missing selections block the transition without
changing the queue. Restore the intended selection or coordinate a revised plan.
Workers require distinct, actual linked Git worktrees in the same common
repository and cannot use the coordinator checkout. The worktree HEAD must
descend from the plan's source and base. Resume rechecks these identities and uses
the original worktree. A worktree already used by an unfinished stream is unavailable, including blocked
or reported work that may need to resume.
These checks establish checkout isolation; they do not establish agent/process
isolation or launch a worker.

## Add-only injection

Supply one ordinary order, plus:

```json
{
  "reason": "A reproduced failure was found within the authorized wave",
  "verified_source": "REPLACE_WITH_CURRENT_COORDINATOR_HEAD"
}
```

The injected order must include a nonempty `investigation` list and a
`verified_source` exactly matching current coordinator HEAD. The HEAD must still
descend from the original planned source. The helper validates this source
identity; it cannot independently reproduce an issue described in prose. The
agent must verify the need and refresh relevant facts before injection.

Existing IDs cannot be replaced. A new order colliding with planned, running,
blocked, reported, or human file ownership needs a dispatch dependency establishing
sequencing. A shared-resource collision with any unfinished order also needs a
dispatch dependency at injection, so injection cannot silently disrupt the
accepted resource map. Scope or ownership changes to existing orders require a
new coordinated plan. There is deliberately no amend, remove, or force command.

## Observed transitions and evidence

The state machine is:

```text
pending --start--> running --report--> reported --integrated--> integrated
pending/running/reported --block--> blocked --resume--> running
```

`start` and `resume` take an actual worktree path. They record the observed Git
identity, configuration digest, and an attempt timestamp. They record a slot
reservation; use the host to start or resume the actual worker. If launching
fails, record the observed blocker and release the reservation. Starting a human
stream may use the coordinator checkout.

A report requires a clean committed source checkout, no caller-reported running
checks, actual matching HEAD/base, and at least one evidence reference:

```json
{
  "summary": "Implemented the bounded change; coordinator review is pending",
  "worktree": "/absolute/path/to/worker-checkout",
  "head": "REPLACE_WITH_ACTUAL_WORKER_HEAD",
  "base_commit": "REPLACE_WITH_PLAN_BASE_COMMIT",
  "checks_running": false,
  "evidence": [
    {
      "kind": "test",
      "reference": "actual runner run ID or evidence artifact path",
      "summary": "Caller-reported check result and any limitations"
    }
  ]
}
```

Reports are appended with their original identity, revision, configuration digest,
and timestamp. Resume or a later report does not rewrite older evidence. The
queue explicitly labels evidence references as caller supplied; it does not
execute referenced checks, verify their contents, declare human enjoyment, or
establish acceptance. Validate runner evidence through `evidence validate RUN_ID`
and apply review/playtest guidance independently. A report may honestly contain
findings or failed checks; the coordinator must resolve those before acceptance.

A block observation is `{"reason": "Concrete remaining blocker",
"checks_running": false}`. It records the caller's claim that checks are no
longer running; it does not inspect or terminate processes. Pending and reported
orders can also be blocked. A blocked agent releases its slot/resources while its
file ownership and history remain part of the plan.

After an authorized integration actually happens, record:

```json
{
  "summary": "Observed integration; combined acceptance evidence is referenced",
  "head": "REPLACE_WITH_CURRENT_COORDINATOR_HEAD",
  "source_head": "REPLACE_WITH_REPORTED_SOURCE_HEAD",
  "evidence_reference": "Actual review and combined-check evidence location"
}
```

The helper requires reported state, satisfied merge blockers, clean source and
coordinator checkouts, unchanged reported source HEAD/configuration, and observed
Git ancestry from source to coordinator HEAD. It records ancestry observation,
not independent verification of review, CI, merge authorization, or combined
behavior. `.gameskills` runtime state is excluded from the untracked-file check.
Tracked modifications, or other untracked files, require committing/cleaning the
actual intended artifact first.

This candidate supports ancestry-preserving merge/fast-forward observations.
Squash/rebase integrations that omit the returned source commit are deliberately
unsupported by this transition: they fail without changing state. Use the normal
PR integration workflow to retain externally verified evidence; do not invent
ancestry, rewrite reports, or mark a queue integrated solely because GitHub shows
a merge. A future GitHub-backed identity path must verify source/base/merge
identity before this helper can record those integrations.

## Persistence and recovery boundaries

The authoritative state is `ROOT/.gameskills/queues/QUEUE_ID.json`. `plan` in that
file is the original validated plan; `orders` is the current authoritative map,
including injected specifications. Status views derive waiting reasons and active
counts from that map. Creation starts at revision 1. Each successful mutation
adds one revision/event. Start attempts, reports, block observations and integration
observations retain their respective source identities and evidence boundaries.

Mutations serialize through a POSIX advisory file lock and write a temporary file,
flush/fsync it, then atomically replace the queue and fsync the directory. Failed
validation never replaces the queue. Rejected stale revisions do not consume
slots. Symlink state directories/files are refused. This implementation targets
POSIX local filesystems (including the supported macOS/Linux environments), not
Windows or filesystems without reliable advisory-lock/atomic-rename semantics.

The queue is locally owned, unsigned JSON, not an adversarial audit ledger.
Manual edits, tools that ignore its lock, or source/configuration changes outside
its observations require coordinator reassessment. Queue events and evidence
are historical records, not proof that all live facts remain unchanged. The
helper has no timed leases or automatic worker-death detection: after interruption,
inspect actual workers/checks and use the explicit block/resume transitions.

Authored plans remain schema 1. Rust queues use schema 2 and runtime `rust`; their
`at` timestamps contain explicit Unix seconds and nanoseconds. Historical Python
queues are read as history and remain immutable to the Rust runtime. Finish an
active old queue before creating or mutating a Rust queue. Setup and mutations
share the queue lock; mutations re-read configuration under that lock to reject
a stale pre-read selection.
