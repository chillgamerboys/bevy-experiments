# Delivery identity and authorization

Record the requested endpoint: design, implementation, reviewable PR, merge or
release. Follow established session authorization and the receiving project's
requirements. Finish useful preparation before seeking any missing final approval.
Creating a PR does not accept it; a successful audit does not authorize merging;
merging does not prove a release was published.

## PR and audit identity

Identify the actual repository/provider, PR URL/number, source branch and HEAD,
target branch and base revision. Include relevant tree, configuration, dependency,
artifact and environment identities for checks. Read current source/CI/reviews
through the configured provider or CLI; do not infer state from a stale comment.
On an ambiguous mutation result, query the actual remote state before retrying.

An audit is an acceptance record, not merely a successful command. Associate each
planned requirement with review judgment, command evidence, documentation checks,
and applicable manual/playtest evidence. Name skipped, pending, failed and
unavailable checks. Evidence reuse follows [verification](verification.md): changed
inputs invalidate affected records; never rewrite old records for a new HEAD.
Where a dependency is uncertain, rerun the check.

Do not post comments, contact reviewers or message others without authorization.
When publishing a PR is authorized, its concrete body describes the final problem,
result and evidence. Respect the receiving project's templates and attribution
rules. For Bevy itself, use the optional contribution workflow: human review and
human-authored upstream prose are required under its receiving policy; ordinary
agent-written PR delivery is not appropriate.

## Integrate and release

Immediately before an authorized merge, confirm the current source/base,
mergeability, required CI, review state, accepted findings and project rules.
Rebase/merge conflicts are a new implementation state requiring affected checks.
Integrate parallel streams serially and verify the combined result. Observe the
remote merge identity and check the resulting target revision. A helper recording
an integration observation does not perform or independently prove the merge.

For a release, verify the artifact users will consume. Record its version/source,
checksums where applicable, selected platform/features and dependency compatibility.
Distinguish source consumption, packaged-artifact installation and actual published
availability. Test from a clean consumer/run location so workspace paths, caches,
assets and locally installed skills cannot conceal missing package contents.

Report the prepared artifact and remaining blocker when publication is outside
scope or not authorized. Preserve unreleased changes and existing releases; do not
delete or replace a published identity to make a failed attempt look successful.
