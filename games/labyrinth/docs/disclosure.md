# Forecasts and viewer knowledge


Labyrinth keeps its combat organization, glyph vocabulary, ability/rank diagrams,
target selection and confirmation local. Flat prototype surfaces are replaceable
appearance, not a contract imposed on other games or future Labyrinth art.

The pure rules package forecasts immediate ordered effects through the same
resolver used by committed actions. A forecast does not consume RNG, roll
initiative, advance a turn, or authorize a command. Labyrinth's presentation layer
separates public authored/base effects from target-specific outcomes and marks
undisclosed information as unknown instead of substituting zero. Periodic effects
describe their timing and remaining opportunities, not guaranteed future totals.
These calculations and disclosure policies do not belong in `bevy-gamekit-ui`.
The current viewer contract leaves identity, allegiance, rank and standing/downed
state public; exact HP amounts, conditions and inspection details can be unknown.
Basic targeting legality can therefore still reflect public standing state.

**Current disclosure is a presentation seam, not network secrecy.** Labyrinth's
multiplayer snapshots still contain the complete combat state, and the normal
encounter is fully revealed. Hidden-information fixtures exercise what the UI can
represent. Actual reveal abilities or concealed enemy facts will require
recipient-filtered snapshots, events and logs before transmission, plus disclosure
tests at that boundary. Hiding a widget or suppressing a log on a client cannot
remove information already sent to it.
