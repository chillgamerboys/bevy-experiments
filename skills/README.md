# Canonical Bevy skill pack

The seven skills in `source/` work in independent Bevy 0.19 games. Shared references
live in `references/`; optional Gamekit guidance loads only when relevant crates
are present. Generated Codex/Claude layouts are not independent prose sources.

- `architect-bevy-game`: architecture and extraction
- `model-turn-based-game`: pure turn-based rules
- `build-bevy-ui`: native presentation
- `test-bevy-game`: deterministic evidence
- `verify-bevy-ui`: static and interactive UI review
- `debug-bevy-runtime`: runtime diagnosis
- `review-bevy-change`: change review

Use the [install](maintainer/install-bevy-skills/SKILL.md) and
[sync](maintainer/sync-bevy-skills/SKILL.md) workflows. They require a real tag/full
immutable SHA, audit before changes, preserve overlays and report generated-file
conflicts. Nothing silently follows latest. Crates and skills initially share a
release tag; future distribution is private initially.

From the repository root, using Python 3.11 or newer (no third-party packages):

```sh
python3 skills/scripts/validate_skills.py
python3 -m unittest discover -s skills/tests -v
```

These are structural/rendering/installer checks, not proof of agent selection or
behavior. Keep API contracts in Rustdoc and adopter conventions in
`.bevy-gamekit/overlays/`, never inside generated skills.
