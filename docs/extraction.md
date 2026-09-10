# Library distribution and optional future extraction

Gamekit and its games stay in this repository. Games consume the library; the first
distribution excludes the games and their assets. Capabilities and canonical skills
initially share one release tag; distribution remains private initially. Consumers
pin tags/resolved SHAs, not branches. A repository split is not a release prerequisite.

The [consolidation plan](gamekit-consolidation.md) defines release gates. Today,
`python3 scripts/check_distribution.py` verifies Cargo-selected package sources with
an external consumer in a temporary library-only workspace. It does not publish or
produce a release artifact; `publish = false` remains in place. The final distribution
channel, internal dependency versions, licensing and clean artifact-install check
remain explicit release work.

## Optional later repository split

The historical recipe below is retained only if a separate library repository is
chosen later. Do not perform it as part of the current consolidation.

Start with a **fresh disposable clone of a committed, flat-layout revision**.
Never filter this working repository or the linked worktrees. Use a pinned
`git-filter-repo` tool installation and review its output before adding any remote.
The [official tool documentation](https://github.com/newren/git-filter-repo/blob/main/Documentation/git-filter-repo.txt)
describes the path filter behavior.

```sh
git filter-repo \
  --path gamekit/crates/ --path crates/ \
  --path gamekit/skills/ --path skills/ \
  --path gamekit/Cargo.toml --path Cargo.toml \
  --path gamekit/Cargo.lock --path Cargo.lock \
  --path gamekit/deny.toml --path deny.toml \
  --path gamekit/rustfmt.toml --path rustfmt.toml
```

Do **not** rename historical `gamekit/` paths during filtering: old commits contain
both nested Gamekit and legacy root manifests, so those destination names collide.
Retain historical layouts; the selected HEAD is already flat. Both old and new paths
are explicit because filtering does not automatically follow directory renames.

Make a normal extraction-finalization commit setting root workspace membership to
`["crates/*"]` and removing game-specific `default-members` and unused game-only
workspace dependency entries. Add a library-specific README, licensing, CI and
ignore rules. Historical commits may contain obsolete members; only the finalized
HEAD is asserted buildable. Run:

```sh
cargo metadata --no-deps --format-version 1
cargo test --workspace --all-features
cargo test --workspace --doc --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 skills/scripts/validate_skills.py
python3 -m unittest discover -s skills/tests -v
```

Let Cargo prune obsolete lockfile entries and commit the result. Check all retained
history for accidentally included games/secrets, then configure the intended private
remote and release tag as an explicit release task, not part of local cleanup.

## Recipe verification

On 2026-09-09, `git-filter-repo==2.47.0` was installed only in a temporary virtual
environment. A disposable two-commit fixture modeled nested Gamekit plus legacy
root manifests, followed by root promotion. Filtering retained both capability/skill
histories, excluded game source from every tree and left the expected flat HEAD.
After manifest finalization, `cargo metadata` and `cargo test --workspace` passed;
the game lockfile entry disappeared. This validates path selection/collision handling,
not a completed extraction or release of the real library. No remote was created.
