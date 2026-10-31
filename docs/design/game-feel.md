# Game feel

How the game should feel in the hands, and the techniques we use to get there.
Movement is a pillar of the game (see [vision](vision.md)),
so this page is about the core of the work, not about polish.

The target is simple to state and hard to hit:
running and jumping around an empty grey room should be fun for ten minutes.
We test against that before we build any content.

## Controls

The game is designed for a gamepad first.
Keyboard is supported and remappable, but every decision is made on a controller.

| Input | Action |
|---|---|
| Left stick, d-pad | Move, aim the Lure |
| South (A / Cross) | Jump, Swoon in the air |
| West (X / Square) | Attack; hold to charge |
| North (Y / Triangle) | Toast, the parry |
| East (B / Circle) | Lunge |
| Right trigger | Lure, held to stay attached |
| Left trigger | Indulge: tap to heal, hold to start a High |
| Right bumper | Plunge |
| Left bumper | Next weapon |
| Select | Map; in the feel lab, back to the spawn point |

The stick reads as digital for walking, with a dead zone and a separate, larger threshold for crouch and look-down,
so a slightly tilted stick never makes you crouch by accident.
The Lure is the one action that reads the stick's angle;
it snaps to the nearest light within a cone around that angle.

Button prompts follow the connected controller's family (Xbox, PlayStation, Switch, generic),
read from the gamepad's vendor id.

## Movement

Every number below is a starting value to tune, not a decision.
All of them live in one tuning file that reloads while the game runs.

The simulation runs at a fixed 120 ticks per second.
A fixed tick makes jumps the same height on every machine,
and lets us record input and replay it exactly when we tune or test.
Presses are collected every frame and handed to the next tick,
so a button tapped between two ticks is never lost.

The techniques we start with:

| Technique | What it does | Starting value |
|---|---|---|
| Acceleration curves | Fast start, short skid when turning, no ice | 0.08 s to full speed |
| Variable jump height | Releasing jump early cuts upward velocity | Cut to 40 % |
| Apex hang | Gravity halves near the top of a jump while jump is held | Within 40 px/s of zero |
| Fast fall | Holding down falls faster | 1.6 x gravity |
| Coyote time | Jump still works just after walking off a ledge | 0.1 s |
| Jump buffer | Jump pressed just before landing still fires, as a full jump; it does not spend the Swoon | 0.12 s |
| Corner correction | Nudge round a ledge corner instead of bonking | Up to 4 px |
| Ledge forgiveness | A dash that clips a ledge top pops you onto it | Up to 3 px |
| Landing recovery | A hard landing squashes the pose and kicks dust, without slowing control | 2 frames of pose |

The first three abilities are movement too, and the feel lab tunes them alongside the rest.

| Ability | What it does | Starting value |
|---|---|---|
| Lunge | A horizontal dash on the ground or in the air, once per jump; a jump out of it keeps its speed | 400 px/s for 0.14 s |
| Swoon | A second jump in the air, with a slower, floatier apex than the first | 370 px/s, apex gravity 0.3 x |
| Cling | Pressing into a wall while falling slides down it; jumping pushes off | Slide at 70 px/s |

Collision is our own kinematic controller against a grid of tiles.
We do not use a physics engine for the player or the enemies.
A physics engine is built to be physically plausible, and a platformer is built to feel good;
the two goals disagree on almost every edge case.

## Impact

Movement and combat feel deliberately different.
Movement is light: everything responds at once and nothing locks you in.
Combat is heavy: swings commit you, and hits land with hit-stop, directional shake and rumble.
[Combat](combat.md) describes the weight in detail; this section covers the shared machinery.

Hit-stop freezes the game for a few frames on contact.
The player's input is still buffered during the freeze, so a press made during it is not lost.

Screen shake uses a trauma value that decays,
with the offset squared so small hits barely move the camera.
It is directional for hits and snaps to whole pixels,
and there is a setting to turn it down or off.

Rumble follows every impact, landing and Lure attachment, each with its own short pattern.
It is part of the feel on a gamepad, not an extra.
There is no rumble on the web build, because browsers do not support it reliably.

## Camera

The camera leads in the direction you face, and settles slowly when you stop.
Vertically it holds still while you jump, and only moves when you land on a new height or fall far;
a camera that bobs with every jump is tiring to watch.

Rooms can define camera zones that lock an axis or frame an arena.
The camera moves smoothly at sub-pixel precision while the world itself stays on the pixel grid;
[art direction](art-direction.md) explains how.

## The mood as feel

The High and the Comedown change feel as well as looks.
During a High, run speed rises a little, the camera loosens, and hits get heavier.
During a Comedown, speeds drop a little, the music sinks, and hits go hollow.

The changes are small, around 10 %, and they never touch jump height or the timing windows above.
Changing the jump arc would break the player's muscle memory,
and the feel of the jump is the one thing we never take away.

## How we tune

Tuning values live in a RON file that hot-reloads, and an in-game panel edits them live.
Changes from the panel can be written back to the file.

A feel lab room holds the standard tests: a long run, gaps of known width, a wall shaft,
a ceiling corner, a staircase of ledges.
Input recordings from playtests can be replayed against new tuning to compare.

Press F1 in a `cargo dev` build for the panel.
`PIXELVANIA_RECORD=path` records every tick's input to a file, and `PIXELVANIA_REPLAY=path` plays it back.
A replay is exact as long as the room and the tuning match the recording.

Headless tests in `crates/world/tests/movement.rs` pin down the numbers we rely on for level design,
so a tuning change that breaks the level metrics fails the build.
With the starting values:

| Metric | Value | Pinned to |
|---|---|---|
| Full jump height | 4.9 tiles | 4.5 to 5.5 tiles |
| Jump and Swoon at the apex | 7.9 tiles | 7 to 9 tiles |
| Tapped jump height | 1.1 tiles | 0.5 to 1.25 tiles |
| Widest gap, running jump | 7 tiles | 6 or 7 |
| Widest gap, running jump and Lunge | 10 tiles | 9 to 11 |
| Ground Lunge | 3.7 tiles | 3 to 4 tiles |

The tests also cover coyote time, the jump buffer, corner correction, the ledge pop,
fast fall, the single Swoon and climbing a shaft with wall jumps.
