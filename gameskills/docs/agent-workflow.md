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
