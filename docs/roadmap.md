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
Sounds are synthesised and wired in, but nobody has listened to them yet;
they were judged by their waveforms and levels alone. The music dip on heavy hits works but has no music to dip.

## Further

Done since: a first finisher (a blow on a staggered guest kills it, with Nectar and slow motion),
a health and poise bar for the player,
and the tile generator, so rooms are painted in stone, wood, velvet and gilt (`cargo xtask tiles`).

Also the candelabrum, the third weapon: a polearm of three burning candles whose grounded swings
leave fire on the floor that burns guests standing in it. Mid-swing, seen edge-on,
its head can turn out of view; it reads best at rest and in wide arcs.

And sound: sixteen effects rendered by `cargo xtask sfx` and played through `kira`,
with heavy hits ducking the music bus and the mood colouring every impact.

And backgrounds: a generated ballroom in parallax layers behind the combat lab,
with a dark foreground passing in front, lifted by the room's lights (`cargo xtask backgrounds`).

And the mood in the picture: the Comedown drains the frame to under half its colour, the High deepens it.
The Toast has a pose, the blade raised like a glass, and a guest's wind-up opens with a glint and a cue.

Next: a first music sketch, for the director to react to.

## For the director

Play the feel lab and the combat lab with a gamepad; both milestones are decided by how they feel.
The combat lab's done test is a fight against the heavy guest with the Jeroboam,
once in neutral or the High and once in the Comedown (F4).

Things only you can judge, because nobody has seen or heard them in play:

- **The sounds.** Every effect was made and checked by its waveform and levels; none has been listened to.
- **The finisher.** Stagger a guest (break its poise with light hits, or a charged Jeroboam slam
  when it is not mid-swing), then hit it: it should die in a burst of pink, with slow motion.
  It works in code, but no scripted run could time it, so it has not been seen.
- **Rumble.** Written for every hit by weight, never felt on a pad.

Decisions taken with a default, to confirm or change:

- **Buttons.** Attack is West (X, Square), the Toast is the left bumper, and North (Y, Triangle) switches weapons.
- **The charge.** Pressing attack swings at once; keeping it held then releases into the charged attack.
  The other way, where holding delays the first swing, makes every attack wait for the release.
- **Hit-stop is per body.** The attacker freezes and the victim shivers, and the rest of the world moves on.
  A global freeze on heavy hits is easy to add if fights feel too busy to read.
- **Impact by mood.** The High scales hit-stop by 1.2, knockback by 1.15, poise damage by 1.2 and shake by 1.25;
  the Comedown by 0.7, 0.75, 0.75 and 0.6. These are starting values.
- **The player's palette.** Burgundy velvet with the rig's pink chest gem kept as a boutonnière.
- **The weapons' looks.** The Jeroboam is a green champagne bottle held by the neck;
  the candelabrum a gilt pole with three candles. Swings reuse the sabre's poses with both hands.
- **The mood's grade.** The Comedown at 45% saturation, the High at 120%.
