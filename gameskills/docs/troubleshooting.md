# Troubleshoot GameSkills

## A skill is absent from the session

Run `gameskills status` to inspect the installed pin and package selection. Canonical
source can be newer than the installed bundle without either being corrupt. Setup
stages instructions; it does not prove an already-running host loaded them.
`gameskills native codex --verify` observes native discovery without a model turn;
Claude launch construction is a separate operation. Use explicit source fallback
only when needed and identify it as such. Do not claim native activation from reading
SKILL.md or silently edit installed caches.

## Local checks pass but delivery is unfinished

Inspect the existing delivery record and actual PR head/base, tracking links, remaining
work and remote checks. Preserve the requested endpoint when resuming. Earlier local
commit-only delivery was an execution failure, not missing permission to publish.
A missing native skill may contribute, but does not alone explain ignoring guidance.

## Evidence becomes stale

Use `gameskills evidence validate RUN_ID` and inspect the named changed inputs.
Source is only one input: configuration, Git refs, environment, executable and managed
files can also change. Preserve the old record and rerun affected checks when required;
do not weaken fingerprints or copy a pass to a new source identity.

## A docs pointer resolves incorrectly

Run `gameskills docs resolve --path PATH` from the adopter root, or provide `--root`.
Paths in `[docs]` and target docs tables are root-relative. Check target component
boundaries and overlapping index directories. Explicit missing indexes are errors;
missing conventional indexes are reported without generating placeholder docs.
A valid file link can still have a broken heading; run the project's docs checker.

## Installation or execution fails

Use `setup --recover` for an interrupted installation transaction; it must not overwrite
subsequent owner edits. Preserve local overlays and old bundles during an explicit
update. Check [installation support](installation.md#compatibility-and-support) before
assuming a platform implements supervision or native verification. Configured commands
run with ordinary process privileges; the CLI is not a sandbox.

## Linear verification is unavailable

Core-only adoption needs no Linear account or key. Connected MCP can perform normal
tracking operations; the optional standalone observer has its own authentication.
A provider error is not proof an issue is absent. Its MCP-first integration remains
open in the reliability plan. Issue deletion is deferred and is not a completion gate.
