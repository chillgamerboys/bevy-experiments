# Delivery audit and candidate limits

Observed September 12, 2026; implementation tracked by HEX-98 and PR #38.

## What failed

PR #37's source guidance already required carrying authorized implementation to a
reviewable PR. `project.delivery_target = "pr"` was configured. The agent nevertheless
stopped at local commits until the user requested publication. Command execution
records verified commands and their inputs, not push or PR existence. There was no
solo-task endpoint record or completion observer. This was an execution failure,
not missing authorization to publish. Linear was a separate missing integration.

That session's native catalog did not expose GameSkills. Canonical skill files were
read directly, which proves source access but not installed skill activation. The
existing runtime can stage selected packages and construct explicit native-client
launch arguments. Staging alone does not alter an already running Conductor/Codex
session. Missing discovery may have contributed; it does not alone establish why
the agent ignored source guidance it had read.

Before adoption, this worktree's installed bundle was `5215cb966addc4f85812dc8f9b11c818ee851946f912733cc2355c743d31a8e2`, pinned to
`0a3d337cd814a2b0392cce7a8b421ecb2448cb31`. The prior embedded bundle had advanced to
`3d876fd981642ff0b7b4cb9dc81270bb9f9a944b828467d38f75896a56c3364a` from
`0531ca2421a04609b2364f631ee9af360f3a68cb`. Both are historical identities, not corrupt
installations merely because canonical source advanced.

## Corrections and evidence boundaries

Core now records solo-task intent, its initial repository/instruction identity,
requested endpoint, configured checks, PR and optional tracker bindings, remaining
work, and fresh observations. Start refuses to overwrite an endpoint on resume.
Check observes GitHub repository, source, base and (for merged PRs) containment of
the merge commit in the remote target. Required tracker observations must match
exact issue/project/PR identities. Provider failure remains unverifiable. Recorded
intent and authorization notes grant no authority and do not replace human review.

The repository's project-owned AGENTS.md gives future sessions an explicit entry
and source fallback when native discovery is absent. Portable plugins retain
self-contained references and supported setup/native activation. No global client
settings or installed caches were edited as source. Helpers cannot force compliance
by an agent that never invokes them.

Evidence fingerprints remain conservative. Git refs, environment, managed files,
source, executables and configuration still invalidate affected records. Validation
now reports changed identity paths without exposing values. The prior same-HEAD
stale event cannot be attributed retrospectively to one input without its original
comparison; refs/environment changes are supported explanations, not proven causes.

Legacy bodies and references were compared with current owners and frozen byte
hashes preserve compatibility inputs. Universal pixel minima and universal
all-feature gates remain intentional retirements. Source inspection, structural
validation and deterministic regressions do not prove forward model parity.

## Validation and rollout limits

Tests exercise delivery/resume, missing/stale provider state, required tracking,
remaining-work handoffs, retention boundaries, reopen/relationship protection,
private export failures, changed snapshots, interrupted deletion and pagination.
Native discovery checks and packaged consumers are reported separately in the PR.
Independent forward model trials are not claimed: this session has no applicable
delegation authorization for evaluators. Existing held-out rubrics remain unexecuted
until those bounded client trials are run. Cost and model-routing comparisons are
unavailable, not zero or automatically improved.

The Linear helper uses the supported public GraphQL schema. Direct authenticated
API and live Hex deletion qualification require the user's separately configured
API key and private durable export location. The connected tools expose no issue
delete operation. No interactive credentials were extracted. Unsupported referenced
binary/document content retains the ticket. Linear exposes no revision-conditional
issueDelete, so a read/delete race remains despite full snapshot rechecking.
Quota recovery is not inferred from deletion; the initial adapter fails unknown
quota-shaped errors closed and routes verified capacity failures through the skill.

Keep this candidate in limited rollout and the issue open while the live pilot,
provider-specific capacity observations and forward behavior remain unverified.
