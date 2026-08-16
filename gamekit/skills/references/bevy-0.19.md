# Bevy 0.19 Working Notes

- Use messages (`Message`, `MessageReader`, `MessageWriter`) for buffered communication.
- Treat `Commands` as deferred. Tests and dependent systems must cross the applicable
  flush/schedule boundary before observing inserted or removed components.
- Use public `SystemSet` seams for consumer ordering; plugin insertion order alone is
  not a durable data dependency.
- Use stable domain IDs for gameplay. `Entity` identifies runtime presentation or ECS
  storage and is not a replay/persistence identity.
- Native keyboard focus uses `InputFocus`, `TabIndex`, and `TabGroup`. Hidden and
  disabled controls must leave the active tab order.
- Resolve responsive UI from logical window dimensions. OS device scale and semantic
  accessibility scale are separate inputs.
- The asset server resolves relative to its configured asset source/root and the
  launched binary's environment, not the source file that names an asset.
- A successful build and clean log do not prove schedules ran, assets rendered, focus
  moved, or the intended application binary launched.
