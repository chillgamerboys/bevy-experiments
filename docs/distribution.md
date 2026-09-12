# Distribution contracts

Gamekit libraries, the GameSkills executable, and its instruction bundle have
separate artifacts and compatibility identities. All packages are currently
unpublished; artifact verification is not a registry release.

| Artifact | Current contract |
|---|---|
| Gamekit facade/capabilities | Opt-in library source; excludes games, game assets and repository tooling |
| GameSkills CLI | Standalone Cargo package with an embedded baseline instruction archive |
| GameSkills instructions | Canonical plugins packaged as an immutable archive; native discovery exposes only selected packages |
| Optional Linear helper | Separate source package; not a core runtime dependency; deletion is deferred |
| Repository tools | Internal `repo-devtools`, not installed by adopters |

The facade has no default capabilities or umbrella plugin. Source consumer checks
build libraries independently of games. Archive checks inspect actual Cargo packages
and build consumers with staged sibling patches; they do not prove registry resolution.

The CLI embeds package-local bundle bytes prepared from a committed canonical
revision. The bundle manifest records file hashes, source identity, catalog version,
selected packages and supported CLI/config/runtime schemas. Setup verifies compatibility,
stages immutable content, and records adoption in the project lock. Updating canonical
source does not silently update installed pins. Rollback is explicit.

## Decisions

Gamekit and GameSkills travel together as development companions, while preserving
separate runtime, library and instruction identities. An instruction edit need not
force a permanent lockstep Bevy/library release. A future repository split keeps
this shared documentation with the companion products.

Source staging, package inspection, native discovery and actual client behavior
establish different claims. Publishing requires its own verified artifacts and
explicit authorization. See [release and split work](plans/distribution.md),
[GameSkills installation](../gameskills/docs/installation.md) and
[packaging commands](../devtools/README.md).
