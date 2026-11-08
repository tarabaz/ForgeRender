---
id: anim.timeline
titolo: Timeline and keyframes
parole: timeline, animation, animate, keyframe, key, keys, interpolation, ease, modifier, wiggle, loop, ping-pong, playhead, work area, fps, duration
ui: panel.timeline
correlate: anim.video, fx.effetti, export.viste
---
# Timeline and keyframes

**Where:** at the bottom of the window, **Timeline** tab next to **Console**. **Detach from the
program** opens it in its own window (also on another screen), **Back into the program**
puts it back.

## Elements and properties

**+ Element** adds what you want to animate: the selected objects, the camera, a light, a smoke,
a dry-ice fog, the ambient haze, the floating dust or the environment. Each element shows its
basic properties (for objects position, rotation, scale and opacity; for the camera position and
target; for lights position and intensity); **+** adds the others one by one (scale per axis,
visibility, color, target, cone angle, field of view, aperture, focus distance, density, HDRI
rotation, brightness). Position, rotation, target and color are one row with three values.

## Keys

- The **diamond** of a row sets or removes a key at the playhead, with the current value.
- When a property already has keys, moving the object with the gizmo or changing the value in
  the panels **records a key** at the playhead.
- Click a key to select it; **Ctrl** or **Shift** adds or removes; drag on the empty area to
  select with a rectangle; drag a key to move all the selected ones (they snap to frames).
- **Delete** removes them, **Ctrl+C** / **Ctrl+V** copy them to the playhead, **F9** applies
  ease in and out.
- **Right click** chooses the interpolation towards the next key: hold, linear, ease in, ease
  out, ease in and out, smooth curve.

## Modifiers

**ƒ+** adds a modifier to a property, applied in order: wiggle (amplitude, frequency, seed),
oscillation, continuous ramp (for example 90°/s for a turntable), loop, ping-pong, offset,
multiply, limits and stepped (stop motion). On a row with three axes the letters X Y Z choose
which ones it acts on.

## Playback

Buttons: start, previous key, play / pause, next key, end, loop. **Duration** (also how long
smoke and fog are computed) and **FPS**; zoom and scroll of time. The **work area** bar above
the ruler (drag its ends or its middle) sets the part that is played and saved.

In the **Viewport** playback is real time; in the **Render** view Play computes each frame with
few samples (20 by default) and moves on: slower than real time, but you see the rendered
animation.

## Effects over time

Simulated smoke and dry ice get their own row: drag its start to choose when the effect begins
and set when the emission stops. What came out keeps moving and fading after the stop.

Every change to keys, modifiers, work area, duration and effect times can be undone with
Ctrl+Z.
