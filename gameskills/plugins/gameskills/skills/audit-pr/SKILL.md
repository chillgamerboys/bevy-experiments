---
name: audit-pr
description: Determine whether a specific PR satisfies its agreed acceptance and receiving-project requirements, with evidence tied to current source and inputs. Use to assemble review, test, docs and applicable playtest evidence; use review for judgment on a change alone.
---

# Audit a specific pull request

Read [project context](../../references/project-context.md),
[delivery](../../references/delivery.md) and
[verification](../../references/verification.md). Resolve the actual repository,
PR, source HEAD, target/base and relevant config/dependency/artifact/environment
identities. Recover the accepted plan and project requirements without adding
unrelated gates. Missing required inputs remain visible.

Map each acceptance claim to relevant review judgment, command results,
documentation checks and applicable playtest/UI/multiplayer evidence. Invoke the
focused skill only for its missing work; share one check graph. Reuse recorded
results only while their inputs remain valid, and rerun when dependency is uncertain.
The runtime's `evidence validate RUN_ID` validates command records; it does not
supply review, manual interaction, human enjoyment or overall audit acceptance.

Inspect current CI and required review state. Resolve findings with the owning
implementer, then refresh source identity and affected evidence. Never relabel an
old record for a new HEAD or count a stopped/skipped command as completed. A docs
change needs no automatic native game walk; a visual/gameplay claim needs evidence
at that level.

Produce a source-bound acceptance record with each requirement's evidence and
status: satisfied, failed, pending or unavailable, plus unresolved findings and
remaining project gates. Conclude whether this PR is ready for the requested next
stage. Continue to `gameskills:merge-pr` only when merge is authorized; a successful
audit may be the complete handoff when another contributor owns integration.

Run the existing task's delivery check with current command evidence. Inspect its
reasons alongside source review and any human/playtest evidence. A provider failure
means unverifiable; it is not proof that an artifact is absent. The checker cannot
attest that a skill was invoked or that a human accepted the change.

Include current-doc accuracy, relevant link/anchor checks and plan retirement in
the acceptance review. Preserve genuinely outstanding requirements with their owner.
