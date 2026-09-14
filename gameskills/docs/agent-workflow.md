# Model routing and task usage

These opt-in commands require CLI 0.1.0-dev.5. They are independent of native
installation: policy resolution does not launch a worker or attest to its model.
Use the host's actual supported models and effort settings when launching.

## Resolve a bounded task

This checkout configures `agents.routing` with small, standard and strong Codex
tiers. Other clients without a mapping remain explicitly unsupported by that
mapping; no Claude execution or pricing parity is claimed. Projects without
routing preserve host inheritance.

```toml
[agents.routing]
default_tier = "small"
escalation_after_failures = 1
max_attempts = 2

[agents.routing.clients.codex.small]
model = "gpt-5.6-luna"
effort = "medium"

[agents.routing.clients.codex.standard]
model = "gpt-5.6-sol"
effort = "medium"

[agents.routing.clients.codex.strong]
model = "gpt-6-astra"
effort = "high"
```

Model IDs above are this project's current choices, not a portable availability
promise. Supply a host capability JSON object with `schema_version: 1`, `client`
and `models`, where each model has `id` and supported `efforts`. Derive it from
the actual host tools rather than copying a stale catalog.

```sh
./target/ci/gameskills agents resolve --client codex --host .context/host-models.json --kind bounded --attempt 0 --reason 'One tooltip timing constant and its affected checks'
```

Attempts are zero-based. The default bounded task selects small; standard and
complex tasks start at their corresponding tiers. The next bounded attempt
escalates; exceeding the configured attempt limit fails explicitly. A `--tier`
override requires a reason. Unsupported model/effort choices fail without a silent
fallback. The host still receives the returned settings explicitly at worker launch.

## Record actual usage

Capture a task baseline before its work, then capture the end counter for the same
thread and attempt. Native Codex checkpoints read bounded session metadata and
counter records; they do not copy conversation content to the report.

```sh
./target/ci/gameskills usage checkpoint tooltip-delay --log /absolute/native/session.jsonl --phase start --role coordinator --attempt coordination
# Execute the task and collect every worker's observed interval.
./target/ci/gameskills usage checkpoint tooltip-delay --log /absolute/native/session.jsonl --phase end --role coordinator --attempt coordination
./target/ci/gameskills usage import --file .context/worker-usage.json
./target/ci/gameskills usage report tooltip-delay
```

Default reports are compact; `--details` adds raw receipts. `observed_span_seconds`
is the covered wall-clock span, while `summed_thread_seconds` includes concurrent
thread time and must not be presented as elapsed wall time.

Generic imports carry source-backed cumulative start/end counters and exact task,
thread and attempt identities. Native checkpoints and imports share an idempotent
ledger under `.gameskills/usage/`; conflicting or overlapping thread intervals
cannot silently count twice. Only measured differences contribute to known totals.
Unknown counters remain visible as unavailable rather than zero. Failed attempts
and coordinator rework belong to the task just as successful worker output does.

Input includes cached input; output includes reasoning output. Their subsets must
not be added a second time. Report requested versus observed model settings
separately. A host model label or worker narrative is not native telemetry.

Optional `usage report TASK --rates RATES.json` uses explicitly supplied dated,
sourced model rates. It reports estimated cost only for known matching data; no
prices are embedded or fetched implicitly. Measure total cost per accepted change,
with token/time/retry and quality context. The first tooltip pilot establishes a
baseline and exercises the workflow; it cannot by itself prove percentage savings.

## Receipt and rate shapes

A receipt has `schema_version: 1`, `task`, `thread`, `attempt`, `client`, `role`
(`coordinator` or `worker`), `start`, `end`, and `evidence_reference`. Optional
`requested` and `observed` objects contain `model` and `effort`. Each counter snapshot
has `at` (Unix seconds) and optional cumulative `input_tokens`,
`cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`. Omitted values
are unavailable. Counter decreases, invalid subsets and overlapping time or known
input/output intervals for a client/thread across tasks are rejected. Adjacent
intervals are allowed. A native interval spanning a model change has no priced
observed model; split work at the change if per-model cost is required.

A rates file has `schema_version: 1`, `as_of`, `source`, `currency` and a `rates`
array. Each entry has `client`, `model`, `uncached_input_per_million`,
`cached_input_per_million` and `output_per_million`. These are caller-supplied
nonnegative rates; the ledger neither authenticates them nor claims billed cost.

## Repository dispatch lessons

Complete active queues under their recorded receiving policy before changing the
project delivery base or instruction pin. Configuration transitions can stale
reports even when worker commits are unchanged; preserve those observations and
use the supported lifecycle to refresh them.

A compact worker brief includes repository lint conventions and affected callers.
Here Rust tests use `expect` and checked access; changes to verification-context
serialization include workflow queue compatibility tests as well as resolver tests.
These small checks caught integration gaps without a game or display sweep.

Shared tooltip timing changes also select consuming-game hover/pin regressions at
normal resolution and a search of consumer guides. Library tests alone miss
consumer frame budgets; numeric and written-out timing references both matter.

## Lean delivery and CI waiting

A settled constant or wording fix uses one delivery record, the relevant source/docs,
one affected check batch and a concise diff review. It needs no separate plan file,
queue or dispatched worker. Choose a short-context small-model session when the host
supports that; resolving a model mapping cannot change an already-running coordinator.
For larger independent work, provide workers exact ownership and executable/check
commands instead of repeating repository discovery. Measure coordinator effort too.

Development CI requires affected regressions (including shared-contract consumers)
and necessary compilation. Broad compile/lint moves to milestone Testing; packaging
and platform matrices move to Release. Reuse the same selected checks and valid
evidence through review and delivery. See [CI policy](../../devtools/docs/ci.md).

Use one `gh run watch RUN_ID --exit-status --compact --interval 30` process with a
local log and host completion notification. Waiting should not cause recurring model
queries or duplicate audits. The host's completion/resume support is an explicit
constraint; the skills cannot add it merely by describing it.

The tooltip pilot exposed excessive coordinator reasoning/polling and missed consumer
timing fixtures. These changes address those mechanisms; measured savings require a
new comparable accepted task. A shorter skill file or passing routing fixture alone
is not a cost result.
