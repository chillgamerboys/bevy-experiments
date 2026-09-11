# GameKit and GameSkills distribution

Status: proposed distribution design, requested September 10, 2026, alongside the
[Rust migration plan](decisions/gameskills-rust-cli.md). No packages or releases have
been published by this work. GameKit, GameSkills and the games stay in this repository;
a repository split is not a release prerequisite.

The [consolidation plan](gamekit-consolidation.md) retains the first private candidate
gate: capabilities and canonical skills share a pinned source revision/release tag,
excluding games and their assets. Consumers pin tags/resolved SHAs, not branches.
Public distribution follows artifact verification and a separate release decision.

## Proposed channels and package boundaries

| Product | Primary distribution | Contract |
|---|---|---|
| GameKit libraries | crates.io source crates, with API documentation on docs.rs | Keep the `bevy-gamekit` facade and opt-in capability crates; default features stay empty. Games, GameSkills and repository tools are not library dependencies. |
| GameSkills executable | A crates.io binary package plus prebuilt archives on GitHub Releases | Working package name `gameskills-cli`, executable `gameskills`. Source installation uses Cargo; prebuilt adoption requires neither Cargo nor Python. No Bevy dependency in the CLI. |
| GameSkills instructions | A baseline bundle embedded in the CLI, plus a versioned standalone bundle archive | Canonical Markdown and native-client metadata remain in `plugins/`. Install the 12 core skills by default and only explicitly selected optional packages. |
| Repository maintenance | Source-built `gamekit-repo-tools`, kept `publish = false` | Layout, CI and distribution checks belong to this repository, not the adopter's runtime. |

These are proposed names, not registry reservations. Settle ownership, consistent
capability names and package metadata in R0/R1 before the first public release.
Keep the existing Rust dependency aliases where possible if public package names
change. Do not create one Rust crate per skill or require a native agent plugin
marketplace for the CLI to function.

GitHub archives are the initial prebuilt channel. Consider Homebrew or winget after
the installation contract and supported platform matrix stabilize; another package
manager is not required for the Rust migration. Game executables and their assets
have their own later release gates.

## A self-contained CLI and explicit bundle updates

Cargo installs executable targets into its binary directory, rather than installing
an adjacent tree of arbitrary resources. Our proposed design therefore embeds a
compatible baseline skill bundle in the executable. A fresh installation can inspect
the catalog and materialize selected instructions without a source checkout or a
network bundle download. See [Cargo installation behavior](https://doc.rust-lang.org/cargo/commands/cargo-install.html).

Keep `plugins/` as the only human-edited skill source. A Rust repository preparation
command generates a deterministic, package-local bundle snapshot and manifest under
the CLI package. The candidate includes that snapshot in the Cargo package and embeds it from
within the package boundary. CI checks regeneration and the content digest. Do not
depend on `../../plugins` existing when compiling an extracted crate, download skills
in a build script, or package the retired Python runtime. Inspect and build the
actual archive using [Cargo packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html).

The embedded payload may contain all six available packages, but installed native
discovery paths expose only the selected packages: 12 core skills, with nine optional
skills available separately. The standalone bundle archive uses the same generated
payload and preserves client metadata; it is not a second authored catalog. An
explicit update may install a newer compatible bundle without replacing the binary.
Never silently track `latest`. Preserve digest validation, path containment, immutable
installation identity, local overlays and atomic recovery across all channels.

## Versions and compatibility

Initially release the GameKit capability family at aligned versions. Record the
supported Bevy version range for each GameKit release; matching version numbers
are not a compatibility test. GameSkills can evolve independently of Bevy and of
GameKit's library version once the coordinated first candidate is accepted.

Record these identities separately:

- GameKit package versions, supported Bevy versions, feature/platform matrix and MSRV.
- GameSkills CLI version, source/build identity and supported host targets.
- Bundle/catalog version, source revision, content digest and supported CLI range.
- Config, queue and evidence schema versions, including supported upgrade paths.

Bundle metadata also declares the Bevy/GameKit versions its instructions cover.
Reject incompatible state before mutation and explain unsupported combinations.
Release notes identify tested native-client versions and distinguish structural
adapter checks from authenticated behavior. A Windows binary does not by itself
establish Windows process-runner support. Coordinating artifacts at one source
revision must not erase their separate identities or require permanent lockstep
releases for every instruction edit.

## Current readiness gaps

`gamekit-repo distribution check` retains Cargo-selected source consumer checks.
`distribution archives` now produces actual Cargo archives in temporary staging,
inspects their contents and normalized manifests, then tests the same empty/pure/UI/
network cases against extracted sources. Staging adds matching versions to internal
path dependencies and omits library lockfiles while siblings are unpublished. The
consumer patches those packages to extracted files and seeds resolution from the
repository lock. Reports retain archive hashes and these transformations, explicitly
setting registry-resolution verification to false. Registry consumers and final
library lockfile packaging remain release gates; public manifests stay unpublished.

At this plan revision, the workspace declares `publish = false`, version `0.1.0`
and `MIT OR Apache-2.0`; no tracked license files were found. Internal library
dependencies are path-only, and package descriptions, readmes, repository links,
MSRV and explicit contents need review across the capability crates. The facade's
current include list covers source, its manifest and README, so license inclusion
also needs an explicit check. The Rust CLI foundation now has an unpublished
`0.1.0-dev.2` manifest and self-contained source packaging. Its archive now includes
`bundle/bundle.json` and `bundle/instructions.tar.gz`, deterministically prepared from
committed canonical instructions and a compatibility declaration. The repository
tool checks regeneration, exports the same bytes as a standalone archive, and checks
the actual CLI Cargo archive against them. This is an instruction preparation format,
separate from the legacy Python bundle format. It preserves native metadata and
core-only defaults and excludes Python code. The Rust runtime installs the embedded
baseline and checks explicit compatibility before setup. Rust queue/evidence schemas
distinguish new observations from historical Python records. Private prebuilt
adoption and source-package builds supply evidence separately from public registry
publishing.

Before publishing, add reviewed license files/notices and metadata, set versioned
internal dependencies, and enable publication only for the intended public crates.
Cargo supports `path` plus `version` for local development and registry use; path-only
dependencies do not establish a publishable dependency graph. See [Cargo dependency
locations](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations).

## Candidate verification and release sequence

1. **During R0/R1:** fix the artifact boundaries, names, licensing/MSRV decisions and
   compatibility manifest. Prove the smallest CLI package builds from its extracted
   archive without sibling workspace paths. Keep release enablement separate.
2. **During R2/R3:** retain empty/pure/UI/network consumer cases, then exercise actual
   Cargo archives and the embedded baseline. For unpublished library dependencies,
   use a controlled staging registry or document the remaining registry check;
   local path rewrites are not registry evidence. Verify selected skill installation,
   compatible updates, incompatible rejection and interrupted recovery.
3. **At R6:** rehearse the complete private candidate from one pinned source revision.
   Test CLI source installation and prebuilt archives on each claimed OS/architecture;
   run the prebuilt consumer with Python and Cargo unavailable. Check checksums,
   source provenance, packaged licenses, baseline setup and runtime/bundle identities.
   Record minimum OS/runtime requirements and real client/runner capability limits.
4. **Before public release:** inspect package contents and run packaging/dry-run
   checks for each public crate. Establish dependency order, including optional
   dependencies referenced by the facade. Pre-publication dry runs cannot resolve
   unpublished siblings from
   crates.io. Version uploads are immutable, so a partial release needs an explicit
   resume/correction decision. See [Cargo publication](https://doc.rust-lang.org/cargo/reference/publishing.html).
5. **Publish only after the release decision:** publish the library graph in order
   and the CLI package, then verify clean registry consumers. Attach verified binary/
   bundle archives and compatibility notes to the release. Verify installed artifacts
   and API docs.
   [docs.rs builds crates.io packages](https://docs.rs/about/builds) in a sandbox
   without network access; configure useful capability documentation and exercise
   those feature selections without game assets or network-dependent build steps.

Evaluate Rust ecosystem tools instead of building a release engine into GameSkills:
[cargo-dist](https://github.com/axodotdev/cargo-dist) for binary archives, checksums
and release workflows, and [release-plz](https://release-plz.dev/docs/usage/release-pr)
for reviewed version/changelog PRs. Keep publishing as an explicit release endpoint;
do not adopt an automatic publication default as part of the tooling scaffold.
Target an allowlist of public packages, never an unrestricted workspace publication.

Release preparation and platform artifact builds run for relevant packaging changes
or an explicit release candidate. Ordinary narrative docs and game changes retain
scoped CI; they do not trigger a complete distribution build. The migration acceptance
requires working artifacts, but does not require a public release or new registry
infrastructure. First-party executable release logic and preparation tools stay Rust;
declarative workflow configuration and external release tools remain appropriate.

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
cargo run --locked -p gamekit-repo-tools --profile ci -- skills legacy
cargo test --locked -p gameskills-cli --profile ci
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
