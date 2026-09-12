# Distribution and repository split

Status: deferred
Owner: shared Gamekit/GameSkills release work. Resume for an explicitly selected release.

Current artifact contracts live in [distribution](../distribution.md). No registry
publication or game release is implied by the following acceptance work.

## Remaining release work

1. Inspect the selected candidate's package metadata, licensing/notices, supported
   Bevy/MSRV/platform matrix and public package names. Library versions currently
   follow the workspace while tool/bundle versions evolve separately; do not assume
   the historical migration version is the release version.
2. Verify every library and tool from actual Cargo/binary archives. Existing library
   consumer staging patches unpublished sibling dependencies; registry resolution
   remains a separate unverified claim. Test empty/pure/UI/network consumers,
   selected instruction installation, updates, incompatible rejection and recovery.
3. Rehearse a private candidate from pinned source. Test each claimed OS/architecture,
   including prebuilt execution without Cargo/Python, packaged licenses, checksums,
   client authentication and actual supported runtime operations. Neither Windows
   installation nor Claude launch construction establishes their untested behavior.
4. Choose distribution channels and release ownership explicitly. Initial candidates
   are Cargo source packages and GitHub binary/instruction archives. Keep the library
   graph's dependency publication order and a bounded public-package allowlist;
   do not publish the whole workspace. A partial publication requires reconciliation,
   not silently replacing an immutable version.
5. Only after release authorization, publish and verify fresh registry consumers,
   installed artifacts and useful public API documentation. Source staging and dry
   runs with local patches cannot stand in for this observation.

Prefer existing release tools when they meet the selected channel's needs, rather
than building a new publishing engine. Ordinary docs/game changes retain scoped CI.
The candidate can be developed and checked without registry infrastructure or a
repository split. Game releases retain their own assets and playability acceptance.

## Future repository split

Gamekit and GameSkills will share one repository; games will become independent
adopters. The root-level organization prepares ownership without creating nested
workspaces in this repository.

When extraction is scheduled:

1. Select a committed revision and work in a fresh disposable clone. Preserve this
   repository and its linked worktrees.
2. Retain `gamekit/`, `gameskills/`, relevant `devtools/`, native marketplace metadata,
   library/workflow documentation and required root configuration together. Inspect
   historical paths as well as current paths before choosing history filters.
3. Give the combined repository its own workspace, lockfile, README and CI. Remove
   game membership/defaults and game-only commands; retain independent capability
   and packaged-instruction consumers.
4. Move each game with its complete source, assets, rules and documentation into its
   adopter repository. Pin library versions or resolved Git revisions and install
   compatible GameSkills artifacts. Local dependency overrides may support joint
   development without becoming required checkout paths.
5. Verify the actual resulting repositories and artifacts. Establish licensing,
   release metadata and publication ownership through the separate release process.

The old flat-library filtering recipe predates the current plugins and Rust CLI
and is superseded. It must not be used to extract the combined product. The
September 9 disposable fixture verified only its historical path-collision example,
not an extraction of this repository.
