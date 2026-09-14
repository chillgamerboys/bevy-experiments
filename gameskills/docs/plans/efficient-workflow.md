# Efficient feature delivery and measured dispatch

Status: active
Owner: GameSkills runtime/instructions and repository CI/adoption.
Endpoint: one framework PR merged to dev after relevant checks, then verified adopter rollout. Main promotions require explicit user approval.

## Accepted contract

Use dev for prompt integration of cohesive feature PRs; worker count does not determine PR count. Agent merges into dev are authorized after required checks and applicable review. Main receives explicit milestone promotions. Preserve the agreed macOS/normal1080 scoped verification, release-only Windows/Linux and milestone-only developer sanity.

Optimize total cost per accepted change while preserving acceptance quality. Record available coordinator and worker input/cached/output tokens, elapsed time, attempts and rework. Missing telemetry or price mappings remain unavailable. Prefer a smaller model for bounded tasks; escalate after a failed bounded attempt or when scope becomes materially ambiguous. Complex shared architecture may select a stronger tier initially with a recorded reason. Do not duplicate every small task with a mandatory strong-model review.

Grill asks one visible decision round at a time. The final message repeats actual unresolved options so disappearing async cards cannot lose the question. Wait for explicit answers before dependent work.

## Implementation and ownership

- CLI model resolver: validated opt-in tier mapping, supported host capability inputs, explicit choice/reason and bounded attempts. It selects policy; the host still launches workers.
- CLI usage ledger: versioned per-task/thread/attempt observations, native Codex checkpoints where exposed, generic receipt import, validated counter deltas, idempotent aggregation and optional explicitly supplied pricing. No fabricated totals or self-reported model claims treated as native evidence.
- Delivery: explicit promotion selection, correct merged-PR ancestry observation from synced source, preserved receiving-branch rigor and source-bound evidence.
- Canonical skills: concise PR lifetime, authorized auto-merge, model routing/escalation, compact worker briefs, task-level reports and visible grill questions. Current docs own local defaults.
- CI/adoption: cancel superseded push runs per branch while retaining required PR coverage; create dev from accepted main, observe real Development routing before setting it as default, publish/install the compatible immutable candidate. Keep this workspace branch name and existing overlays/history.

## Verification and rollout

Use focused model/usage/delivery/config/help tests, CLI lint, canonical skill/docs/format checks and relevant CI routing tests. Validate actual bundle/archive and supported local adoption; do not claim that a running session has reloaded instructions. Record native selection limits explicitly. No game/UI sweep belongs to the framework change.

After framework rollout, run a separate small tooltip delay pilot using the small tier and fresh usage checkpoints. Increase pin delay from one second; two seconds is the suggested value to settle before the pilot. Preserve hover preview, pinned persistence, close and menu behavior. Run only relevant timing/tooltip coverage and report measured task cost/tokens/time without an unsupported savings percentage. More challenging real bugs follow this baseline pilot.

Retire the completed UI implementation plan and repair links after preserving remaining evaluation and rollout work in their owners. This plan remains active until framework rollout and the separately authorized pilot handoff are observed.
