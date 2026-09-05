# Evidence Boundaries

| Evidence | Can prove | Cannot prove |
|---|---|---|
| Pure/domain test | rules, invariants, deterministic transitions | plugin wiring or presentation |
| Minimal Bevy app | resources, messages, schedules, deferred commands | renderer output or feel |
| Structural UI snapshot | hierarchy, semantics, focusability, layout facts | gameplay truth, pixels, motion |
| Deterministic frame | static presentation, contrast, clipping, composition | interaction, timing, authority |
| Interactive walk/video | input paths, focus, scrolling, motion, feel | exhaustive domain correctness |
| Clean log | absence of recorded diagnostics | successful rendering or gameplay |

Choose evidence from the claim. For merge-bound UI work, combine structural tests,
static frames, and an interactive walk. Never derive gameplay acceptance from text
that the UI rendered about itself.

For multiplayer, keep these claims separate: a discovery observation proves listing;
an authenticated connection proves transport/admission; targeted snapshots prove
disclosure only when inspected per recipient; and multi-app gameplay tests prove
authority/lifecycle behavior. An in-memory link proves only the code using that link.
A same-machine socket test exercises the transport stack but cannot prove another
machine's firewall, interface routing, or multicast path. Real cross-machine mDNS and
tailnet walks are integration evidence, not replacements for deterministic provider
tests, and deterministic tests are not substitutes for those walks.
