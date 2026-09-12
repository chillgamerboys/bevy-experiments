# Work with GameSkills

Start at `plan` for implementation or use a focused skill for a narrower request.

## Work and evidence

Use `plan` for a complete scoped workflow or invoke a focused toolbox skill for a
bounded request. Creative levels are implement, refine, co-design and explore;
level 2 is the default. The worker cap is five and the host may support fewer.
Configured capacity does not grant delegation, publication or merge permission.

`gameskills.toml` owns literal command argument arrays, prerequisites, exclusive
resources, target paths and worker limits. Run only the checks relevant to the task:

```sh
gameskills run repo-check tooling-test --max-workers 1
gameskills evidence validate RETURNED_RUN_ID
```

Full contracts and JSON examples live with the portable core:

- [Work orders and queue transitions](../plugins/gameskills/references/work-orders.md).
- [Execution, resource locks and evidence](../plugins/gameskills/references/verification.md).
- [PR delivery and integration](../plugins/gameskills/references/delivery.md).

Queues validate real linked worktrees, ownership and observed integration. They do
not launch models. `dispatch` uses the host's worker mechanism with actual supported
model/effort settings. Inherit session defaults unless an authorized mapping exists.
Worker reports remain caller-supplied references, separate from runner observations.
A passing command alone does not establish visual quality or release readiness.


## Documentation context

Run `gameskills docs resolve --path games/example/src/lib.rs` to locate root and
component entrypoints. Repeat `--path` for mixed work. Read the relevant architecture,
Decisions, development, testing or troubleshooting sections; the resolver does not
read the pages for you. Missing conventional docs are diagnostic; broken explicit
mappings must be repaired. Proposed plan text does not replace current contracts.

## Delivery

Start or resume the existing `gameskills delivery` task before implementation.
Preserve its endpoint across interruptions. Bind the actual PR and optional tracker;
observe the remote head/base and relevant checks before reporting delivery. A command
record is not a PR, a loaded skill, a review acceptance or a release.

When behavior changes, update its current docs and any skill pointers in the same
work. Preserve unfinished requirements in their active owner plan. Remove completed
plans and repair links after the whole outcome is settled. Keep consequential rationale
in the relevant current guide's Decisions section, rather than an archive of plans.

Linear is optional. Use connected tools for ordinary issue lookup, updates and PR
links. Deletion and the live Hex pilot are deferred; no private export setup is needed.
This repository's existing standalone tracking observer still needs separate credentials;
its MCP-first replacement remains [open work](plans/workflow-reliability.md).
