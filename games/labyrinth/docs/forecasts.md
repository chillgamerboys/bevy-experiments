# Forecast verification


The shared contextual-help tests are independent of Labyrinth. They exercise
focus/pointer precedence, modal scope/restoration, hidden/disabled/removed sources,
clipping, activation non-interference and unchanged-resource detection. Their
explicit `Interaction` and geometry fixtures prove selection mechanics, not native
cursor hit testing or rendered tooltip placement. Each adopter must also exercise
real input and layout through its production plugin stack; helpful text appearing
does not prove that the associated action can be selected and confirmed.

Keep Labyrinth's forecast evidence at two separate levels:

- Pure rules: base power versus effective damage and actual HP loss, shared
  immediate resolution, status application/removal, position changes, no mutation
  or random/turn advancement, and off-turn previews granting no commit authority.
- Presentation: known versus unknown HP, modifiers and details; uncertainty text
  and absent exact projections; conditional periodic-effect explanations; and
  matching disclosure in labels, inspection, logs and contextual information.

Vary concealed inputs while keeping public facts fixed and compare the resulting
presentation, including error shape and derived values. Test partial disclosure,
not only an entirely concealed actor. Separately review projected HP segments,
pending effect markers and confirmation clarity at all supported canvas sizes.
Normal encounters remain fully revealed: a hidden-information fixture is neither
an implemented reveal ability nor evidence that network payloads are filtered.

No automated selection test, snapshot or forecast parity check establishes the
feel of the dock. A pointer/keyboard walk still checks hover-to-focus transitions,
off-turn inspection, ability -> target -> Confirm, modal return, overflow and
resizing. Record any missing interactive or cross-machine evidence explicitly.

Tooltip lifecycle regressions include immediate first-frame preview and departure,
one-second continuous hover to lock, persistence over empty space, source switching,
explicit keyboard inspection, modal cleanup, and ×/Escape/outside dismissal.
The native-layout test compares the card rectangle on every frame across locking:
the preview must use the same shorter geometry as the locked card, not reserve an
extra footer. Native pointer tests close the × over an underlying character and
verify that stationary-pointer dismissal does not reveal a new tooltip. Render
`labyrinth_review ... 1280 720 auto help` and `help-locked` for separate authored
presentation states; those captures freeze timing and do not prove hover duration.

See [Labyrinth](testing.md),
[Carterfight](../../carterfight/README.md) and
[deckbuilder](../../deckbuilder/README.md) for game-specific acceptance.
Skill validation proves structure/rendering parity, not agent selection behavior.
