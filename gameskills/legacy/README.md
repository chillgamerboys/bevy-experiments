# Legacy skill compatibility

This directory preserves the seven-skill source and references used to validate
existing adopters. It is frozen compatibility material, not the current catalog.
The retired install/sync skills redirect adopters to the Rust migration path.

Use [current GameSkills](../README.md) for development and installation. Run
`cargo run --locked -p gamekit-repo-tools --profile ci -- skills legacy` from the
repository root to validate this source. Existing installed overlays and snapshots
remain owned by the adopter.
