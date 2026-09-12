# GameSkills architecture

The executable and immutable instruction bundle are separate identities. The core
package supplies focused development workflows; optional plugins add domain guidance.
Gamekit is optional to an adopter. The [catalog](catalog.md) owns skill boundaries.

## Runtime ownership

- `cli/src/config.rs`: project configuration structure and defaults.
- `cli/src/docs.rs`: read-only documentation discovery, independent of installation.
- `cli/src/installation/`: bundle verification, atomic setup, native adapters and migration.
- `cli/src/workflow/`: revision-guarded work queues; queues do not launch agents.
- `cli/src/runner/`: configured commands, supervision, resource locks and evidence.
- `cli/src/delivery.rs`: solo intent and observed PR/tracker delivery.
- `plugins/`: canonical skill bodies and package-local references.
- `linear/`: optional provider helper; no core dependency.

These paths are relative to the GameSkills source directory. The modules support
the executable; they are not a promised stable public Rust library interface.

## Decisions

Project configuration owns commands, targets and local workflow choices. Skills
retain triggers, necessary judgment and completion criteria; current project docs
own local facts. Resolve docs from the adopter root and APIs from its actual locked
dependencies, not this development checkout or sibling plugin-cache paths.

Native packages remain self-contained. Portable references carry reusable craft;
project indexes route to local architecture, development and troubleshooting. A
pointer states when to read a source, what to establish, and the next action.

Setup preserves project-owned instructions and overlays. Immutable bundles, recorded
pins and command evidence are not authoring locations. Keep historical evidence
unchanged; source/config/ref/environment changes may invalidate a current claim.
The validator names changed input categories without exposing their secret values.

No helper forces an agent to invoke a skill or finish its task. Native discovery,
model behavior, command success, PR publication and human acceptance are different
observations. [Contributing](contributing.md) describes their verification.

## Documentation discovery

Optional `[docs]` and `[targets.NAME.docs]` tables accept `index` and `plans`, each
relative to the repository root. Target mappings require a target `path`. Defaults
look for `docs/README.md`, then `README.md`, at the root and the selected target.
A plans location may be absent until a substantial plan exists.

The most specific enclosing target wins by path component. Declared index parent
folders and plan folders also route changes to docs kept outside the source tree.
Equal target paths or equally specific competing docs owners are errors. A mapped
root README identifies that file, not every file in the repository. Multiple input
paths return a deduplicated set of indexes while retaining the root owner.

Explicit indexes must be files, plans locations directories when present, and
existing symlink ancestors must stay inside the repository. Cross-owner links inside
the repository are allowed. Missing conventional indexes are diagnostics rather than
invented files. The command returns locations only and performs no network calls.

Older configuration remains supported. Mappings require CLI 0.1.0-dev.3 or newer;
an older CLI rejects the unknown field. Upgrade explicitly before adding mappings.
