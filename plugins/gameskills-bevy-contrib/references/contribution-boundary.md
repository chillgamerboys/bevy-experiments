# Selective engine contribution preparation

Most findings belong in the game, GameKit, ecosystem packages, documentation or
skills. Investigate upstream only for a verified expected-behavior defect or a
compelling engine-level capability gap. Convenience, novelty, popularity and reuse
alone do not justify engine ownership. A tooltip package can stay in GameKit while
a separately verified engine input defect is fixed upstream.

For a bug, preserve expected/actual behavior, exact Bevy revisions, a minimal
engine-only reproduction, relevant platform/features and a regression check where
appropriate. Compare the supported release and current upstream state when useful;
search existing issues/proposals before duplicating effort. For a capability gap,
identify the actual game need, existing APIs/plugins/plans, who benefits and why
engine integration is worth its compatibility, build and maintenance costs.

Use private technical artifacts for a human contributor's investigation and
understanding. Before an upstream issue, proposal or PR, a human must review the
evidence/rationale and proposed design/code. General authorization to help Bevy is
not proof this review happened. Track a local workaround and removal condition;
engine acceptance is not a dependency for releasing a useful local package.

## Receiving policy

Recheck Bevy's current [AI policy](https://bevy.org/learn/contribute/policies/ai/)
when preparing work. Checked September 10, 2026: it permits careful human-led
assistance in some code-related work, requires human understanding/ownership,
disclosure and human commit authorship, and prohibits AI-generated public prose,
communications and media. Human contributors must author upstream issues, PR
bodies, documentation and communications themselves. Do not draft ready-to-post
upstream prose or mark agent-produced work as human-reviewed.

Consult current [upstreaming guidance](https://bevy.org/learn/contribute/project-information/upstreaming/)
for the engine-level rationale and
[contribution process](https://bevy.org/learn/contribute/helping-out/opening-pull-requests/)
for the project's expected discussion/review path. Larger changes need human-led
design discussion before speculative implementation. Acceptance belongs to Bevy's
maintainers. When an upstream code example is relevant, read the current
[example guidelines](https://bevy.org/learn/contribute/helping-out/creating-examples/);
keep an engine-only reproduction free from ecosystem dependencies.

This package prepares technical evidence and supports human understanding. It
does not invoke ordinary automated PR prose/publication for Bevy or contact its
community. Continue authorized local investigation while any final human review
or submission remains outstanding, and name the exact remaining handoff.
