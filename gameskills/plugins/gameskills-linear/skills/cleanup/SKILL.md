---
name: cleanup
description: Assess an explicitly requested Linear issue cleanup using the current project policy and available connector capabilities. Deletion is deferred unless the user selects that work; installing the plugin never schedules it.
---

# Assess requested cleanup

Read [project context](../../references/project-context.md) and the adopter's current
tracking/retention policy. Confirm the requested project and unfinished work that
must remain. Use the connector's declared tools; never invent a deletion operation
or assume moving to Done reclaims issue capacity.

Deletion is deferred in this candidate. If requested later, inspect current supported
capabilities and report a concrete manual action when the connector cannot delete.
Do not request a backup directory or separate key merely to use ordinary tracking.
The [helper status](../../references/cleanup.md) describes an earlier prototype;
it is not the current policy or permission to invoke deletion.

No ticket changes follow from a plan-only cleanup discussion. An actual authorized
operation must preserve project scope, verify current state and report selected,
deleted, skipped or unverifiable results honestly. An unavailable lookup cannot
establish deletion. No scheduled job is installed.
