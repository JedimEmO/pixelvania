# Roadmap

What we build next, in order, and what is waiting for the director.
The milestones and their done tests are in the [plan](plan.md);
this page breaks the current ones into steps small enough to finish and check one at a time.

Each step ends with the workspace tests, clippy and rustfmt passing,
something rendered or recorded that shows it working,
and a commit on `master`.
A step that needs a decision only the director can make
goes on with a reasonable default, and the question goes under *For the director*.

## Finishing pixelfx in Rust

1. **Effects in the game.**
   A Bevy plugin runs pixelfx effects on the fixed tick, at the effect's position in the room.
   Colour goes to the colour buffer, light to the light buffer,
   and particles collide with the room's tiles.
   Landings raise dust, hard landings the heavy landing, and a dev key spawns any effect.
   Done when a slash lights the walls around it and its sparks bounce off the real floor.

## Combat lab

2. **The player as a rig.**
   The grey rectangle becomes the pixelfx rig, drawn every frame,
   with poses for idle, run, turn, jump, Swoon, fall, land, Lunge and wall slide.
   Squash, stretch and the afterimages keep working.
   Done when a contact sheet of every pose and in-game screenshots read clearly at 1x.

3. **Combat core.**
   In `world`, without Bevy: attacks as frame data in RON
   (wind-up, active, recovery, cancel window, hitboxes, hit-stop, poise damage, knockback),
   an attack state machine with an input buffer, hit detection,
   and bodies with health, poise, stagger, launch and wall splat.
   Done when headless tests pin commitment, the Lunge cancel, poise breaks and wall splats.

4. **The sabre.**
   The three-hit combo, charged attack, rising attack, air combo and pogo,
   with the swing poses driven by pixelfx's act timing.
   Done when a contact sheet shows every move's wind-up, release and recovery.

5. **Impact.**
   Hit-stop, shake along the line of the hit, rumble by weight,
   hit sparks and slashes from pixelfx, and the victim's shiver.
   Done when a recorded hit shows freeze, shake and sparks on the right ticks.

6. **Three enemies.**
   A light, a medium and a heavy guest on the shared rig,
   each with readable attacks, poise and its weight class's reactions,
   in a combat lab room. The Toast parries their blows.
   Done when each enemy can be fought and killed, and the tests cover their reactions.

7. **The Jeroboam.**
   The second weapon, switched at any time: slow, huge poise damage, the heaviest hit-stop,
   and the charged ground slam.
   Done when the heavy enemy staggers from it and not from the sabre's light hits.

8. **Combat tools.**
   Hitbox, hurtbox and poise overlays, pause and single-tick stepping,
   an enemy spawner and frame data reloading while the game runs.

9. **The mood switch.**
   High, neutral and Comedown as multipliers on hit-stop, knockback, poise damage and shake,
   switched from the panel, so the combat lab's done test can be played.

Steps 2 and 3 do not depend on each other, and step 3 may be handed to Codex.

## Further, if there is time

The finisher, the candelabrum,
and the tile generator that starts the *One finished room* milestone.

## For the director

Play the feel lab and the combat lab with a gamepad; both milestones are decided by how they feel.
