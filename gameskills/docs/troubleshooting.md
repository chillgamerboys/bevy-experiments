# Troubleshoot GameSkills

## A skill is absent from the session

Run `gameskills status`. Bundle validity and `native_clients.codex.registration`
are separate: `missing`, `outdated` or `conflict` is an actionable host-registration
gap, even when the pinned bundle is valid. An older installation may have staged
all files without making them discoverable to an ordinary host. Repair the same pin
with `gameskills native codex --register --apply`, then observe ordinary discovery
with `gameskills native codex --verify-project`.

For `conflict`, inspect the reported project setting or interrupted transaction;
GameSkills will not overwrite a newer local edit or silently enable a disabled
plugin. Use `native codex --register --recover` for an interrupted targeted repair,
or `setup --recover` for an interrupted installation. A copied checkout's absolute
marketplace path is `outdated`; re-register after its pinned bundle is hydrated.

If project registration is present but ordinary discovery fails, check whether the
project is trusted, the host ignores/overrides project configuration, or its Codex
version lacks the needed plugin support. GameSkills does not change global trust
or retry with injected flags. `native codex --verify` uses explicit session overrides
and can pass while ordinary-host discovery fails. A missing executable is distinct
from either result. Claude persistent registration is not yet supported.

Start a new Codex/Conductor session or restart its host to load changed registration.
An already-running session's catalog is a separate observation. Canonical source can
be newer than the installed bundle without corruption. Explicit source fallback can
continue useful work, but does not close the registration/discovery defect; reading
SKILL.md is not native activation. Preserve installed caches and the recorded pin.

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

If another worker's unrelated branch is the only changed input, commands with
known Git dependencies can opt into an exact `git_refs` list in project config.
Keep the review base and any other consumed refs; default/`"all"` dependencies keep
the whole graph conservative. Use the current development runtime and create new
evidence after the configuration change. See [declared Git inputs](../cli/README.md#declared-git-inputs-for-command-evidence);
this does not make an old invalid record valid.

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
