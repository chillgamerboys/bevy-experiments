# Evolving an optional GameKit capability

Start with demonstrated game needs, the owning source and current consumer behavior.
A stable reusable contract can justify extraction; identical-looking code alone
cannot. A second consumer is valuable transfer evidence, not a mandate to invent
one before shipping a small useful package. Bevy is the engine foundation and
GameKit uses its concepts/extension seams; keep the game's composition root local.

Inspect actual crate Rustdoc, public API, examples, features and tests from the
resolved revision. Keep public contracts there instead of maintaining a second API
manual in skills. Useful areas to inspect include pure identities/algorithms,
roster/cursor sequencing, UI mechanics, session/admission, discovery and transport.
Check for capabilities already available in Bevy or the ecosystem before inventing
new ownership. Existing convenience does not imply an engine bug or gap.

Define opt-in behavior, dependency direction, extension/scheduling seams, failure
semantics, lifecycle and supported version/features before reshaping code. Shared
crates cannot import consumer games, encode their rules/branding or start services
merely because a facade is selected. Pure capabilities should remain usable without
windowing/networking dependencies when that is their promise.

Validate retained and changed contracts in capability tests plus affected consumer
seams. Exercise minimal and promised feature combinations, independent source and
packaged-artifact consumers separately, and realistic examples. Two games compiling
does not establish shared UI feel; a source probe does not prove published install.
Include migration, compatibility and removal conditions for temporary shims.

Ordinary extension packages can remain maintained here. A verified engine defect
or compelling missing engine-level capability can be investigated through the
optional contribution package, with human review before upstream pursuit. There
is no transfer quota and no Bevy acceptance dependency for a GameKit release.
