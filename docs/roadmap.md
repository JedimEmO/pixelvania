# Roadmap

What we build next, in order, and what is waiting for the director.
The milestones and their done tests are in the [plan](plan.md);
this page breaks the current ones into steps small enough to finish and check one at a time.

Each step ends with the workspace tests, clippy and rustfmt passing,
something rendered or recorded that shows it working,
and a commit on `master`.
A step that needs a decision only the director can make
goes on with a reasonable default, and the question goes under *For the director*.

## Where the combat lab stands

Steps 1 to 9 below are built, and each one works in its basic form.
None of them has been played with a gamepad yet, which is the test that decides them.
Run `cargo run --bin pixelvania`, press Start or Tab for the combat lab,
and see the README for the controls and the debug keys.

1. **Effects in the game.** Done.
   Effects land on the room's floors, hide behind its walls, and light what is around them.
   Landings, jumps, wall jumps and ground Lunges raise dust; a fast-fall slam cracks the floor.
2. **The player as a rig.** Done.
   Ten movement states in the `anim` crate, drawn every frame; `cargo xtask rig player` shows them.
3. **Combat core.** Done, by Codex, in `world::combat`, with 28 headless tests.
4. **The sabre.** Done.
   Combo, charged, rising, air combo and pogo, posed from the frame data.
5. **Impact.** Done.
   Per-body hit-stop, the victim's shiver, shake along the hit, rumble by weight,
   slash trails, sparks, a white flash on the body hit, and wall splats.
6. **Three enemies.** Done in a first form.
   Light, medium and heavy guests with their own rigs, weapons and frame data,
   who walk up and swing when in reach, each with its own habit of spacing.
7. **The Jeroboam.** Done: a champagne bottle as long as the player is tall, a new rig weapon.
8. **Combat tools.** Done: F2 and F3 pause and step, F5 shows boxes and bars,
   7, 8 and 9 spawn guests, and frame data reloads with the `dev` feature.
9. **The mood switch.** Done: F4 or the panel switches neutral, the High and the Comedown.

## Rough edges we know about

The player is the stock mannequin in a burgundy suit;
the costume silhouette the art direction asks for, a coat or hair, needs new rig parts.
Dust from running and jumping is faint in dark rooms.
Guests do not jump, block or circle. They approach and swing; the light guest then skips back,
and the medium guest keeps its spear's distance.
There is no sound yet, so the sub-bass thump and the music dip on heavy hits are missing.

## Further, if there is time

The finisher, the candelabrum,
and the tile generator that starts the *One finished room* milestone.

## For the director

Play the feel lab and the combat lab with a gamepad; both milestones are decided by how they feel.
The combat lab's done test is a fight against the heavy guest with the Jeroboam,
once in neutral or the High and once in the Comedown (F4).

Decisions taken with a default, to confirm or change:

- **Buttons.** Attack is West (X, Square), the Toast is the left bumper, and North (Y, Triangle) switches weapons.
- **The charge.** Pressing attack swings at once; keeping it held then releases into the charged attack.
  The other way, where holding delays the first swing, makes every attack wait for the release.
- **Hit-stop is per body.** The attacker freezes and the victim shivers, and the rest of the world moves on.
  A global freeze on heavy hits is easy to add if fights feel too busy to read.
- **Impact by mood.** The High scales hit-stop by 1.2, knockback by 1.15, poise damage by 1.2 and shake by 1.25;
  the Comedown by 0.7, 0.75, 0.75 and 0.6. These are starting values.
- **The player's palette.** Burgundy velvet with the rig's pink chest gem kept as a boutonnière.
