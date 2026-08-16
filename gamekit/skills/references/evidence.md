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
