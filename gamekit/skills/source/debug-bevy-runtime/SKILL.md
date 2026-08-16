---
name: debug-bevy-runtime
description: Diagnose Bevy 0.19 failures that compile but behave incorrectly at runtime, including missing assets, wrong asset roots, stalled states, silent defaults, schedule ordering, deferred commands, focus, rendering, feature flags, or plugin composition. Use when tests or logs disagree with observed game behavior. Do not use for an ordinary compiler error with a direct diagnostic.
---

# Debug Bevy Runtime

Trace the failed fact back to its authoritative producer before changing code.

Read `.bevy-gamekit/overlays/debug-bevy-runtime.md` first when it exists; treat it as adopter context without accepting silent failure.

## Workflow

1. Reproduce with the exact binary, working directory, features, configuration, route, and input sequence.
2. Identify the first missing or incorrect authoritative fact. Do not begin at the final visual symptom.
3. Trace plugin registration, resource initialization, state entry, system-set ordering, run conditions, messages, and deferred command application.
4. Verify asset-server root and source binary. A file existing elsewhere in the repository proves nothing.
5. Distinguish absent, loading, failed, and ready states. Reject silent fallback from load/config failure to initial state.
6. Add temporary observation at owner boundaries, then encode the reproduced failure at the narrowest deterministic test altitude.
7. Remove diagnostic-only changes after the permanent assertion exists.

For multiplayer, locate the failing layer before changing code: provider discovery,
route/firewall, encrypted transport, admission, game authority, or targeted snapshot.
Do not diagnose mDNS across a tailnet; inspect the explicit tailnet provider and its
fixed CLI status input instead.

Read `../../references/bevy-0.19.md` for common version-specific traps and `../../references/evidence.md` before treating clean logs, screenshots, or tests as proof. If Gamekit crates are present, read `../../references/gamekit-apis.md` for their ownership seams.
