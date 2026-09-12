---
name: debug
description: Investigate a Bevy failure or unexplained behavior, including runtime wiring, assets, states, input and integration. Use when the cause needs reproduction or tracing; a direct compiler diagnostic usually needs no debugging workflow.
---

# Debug the observed failure

Read [project context](../../references/project-context.md) and relevant
[Bevy craft](../../references/bevy-craft.md). Capture the exact source/binary,
working directory, features, configuration, route and input sequence. Establish
expected and observed behavior before claiming a defect is reproduced.

Trace the first wrong authoritative fact back to its producer. Inspect plugin
registration, resources, states, run conditions, system ordering, buffered messages
and deferred commands as relevant. A late visual symptom may have a domain or
asset-loading cause. Verify the launched binary and configured asset source;
a file existing elsewhere is not evidence it loaded.

Use bounded observations at ownership seams to distinguish competing causes.
Keep absent/loading/failed/ready states explicit; do not convert a failure into
a silent default. Once the cause is established, make the authorized correction
and add the narrowest meaningful regression check. Remove temporary diagnostics
that no longer contribute to the result.

For networking, resolve discovery, route, transport, admission, authority and
recipient disclosure separately using the selected multiplayer guidance. mDNS
across a tailnet is not the tailnet provider's discovery test. For UI, use the
selected package to separate structure, pixels and native interaction.

Report reproduction, causal evidence, correction and regression results, or the
remaining hypotheses and blocked observation. A suspected engine bug remains a
candidate until isolated against a known Bevy source; select optional contribution
preparation only if a verified bug or compelling engine-level gap merits it.

Use the affected owner's troubleshooting and architecture guidance to establish
expected behavior. If the fix invalidates a diagnostic recipe, repair that recipe.
