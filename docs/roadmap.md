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

The player is the stock mannequin in a burgundy tailcoat, whose tails are the first costume part on the rig;
hair, masks and hats would push the silhouette further.
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

And a first music sketch: the ballroom as 64 seconds of Italo disco at 120 BPM in D minor,
five stems that the mood mixes: everything up in the High, drums and lead gone in the Comedown.
Its stems are rendered, not committed: run `cargo xtask music` once after cloning.

## One finished room

The director has played the combat lab and signed off on its feel.
This milestone is one room of the ballroom at final quality, built from those pieces.
Its five steps are built; what decides it now is playing the room and listening to it.

10. **The Lure.** Done. The right trigger throws a rope to the chandelier or lantern the stick aims at,
    within reach and in sight, and the body swings from it as a pendulum:
    pumped by the stick, climbed with up and down, flung on when let go.
    A rope snapping taut keeps the body's speed, turned along the swing, so momentum carries through a catch.
    Taking hold gives back the Swoon and the Lunge. Seven headless tests in `crates/world/tests/lure.rs`.
11. **The chandelier.** Done, and a lantern with it, painted by `assetgen::hanging`
    and repainted at each angle as they sway, so every pixel stays whole.
    They lean towards a body on their rope and flare when the Lure takes hold.
12. **Costumed guests.** Done, by Codex: hats, masks and hair on the rig.
    The light guest wears a bouffant and a domino mask, the medium a beak mask and a plume,
    the heavy a top hat. `cargo xtask rig` shows them.
13. **The progression fade.** Done, set by hand: F6, a slider in the F1 panel, or `PIXELVANIA_FADE`.
    Lit surfaces go grey while lamps, flames and every glow keep their colour,
    the High gives back four fifths of it, and the fade thins the music's warm stems.
    The first version greyed only the surfaces' own colour, which the coloured lights then tinted again,
    so in play it barely showed; it now applies after lighting.
14. **The room.** Done in a first full form: `assets/rooms/ballroom.room`, where the game now starts.
    A dance floor whose lights pulse on the music's beat, an orchestra pit with two chandeliers over it,
    a lantern-lit shaft, a gilt gallery, and a lounge reached by one last swing.
    `crates/world/tests/ballroom.rs` finds a way through each route against the real room and tuning,
    so a change that breaks one fails a test.

## The vertical slice

The ballroom as a player reaches it in [progression](design/progression.md):
entered with the Lunge, Cling, the Swoon and the Lure, at a fade of 0.26.
First the systems it needs, then its rooms.

15. **Doors and the world file.** Done. `assets/world.ron` lists the rooms and the start;
    doors are room entities (`door 43 18 h=3 id=east to=ballroom:west`),
    and walking into one draws a black curtain, loads the next room,
    and puts the player just inside the matching door with their running speed.
    A test checks that every door leads to a door that leads back, and that nobody arrives inside a wall.
    The antechamber, where a new game starts, opens into the ballroom.
16. **Progress.** Done. `world::Progress` records the abilities and weapons found and the pickups taken;
    the fade follows the abilities by their steps, unless set by hand with F6.
    Pickups are room entities (`pickup 7 7 item=candelabrum`), a glowing diamond that bobs,
    taken with a burst, a sound and a banner naming what it gives.
    North only steps through weapons found. The candelabrum waits in the ballroom's lounge.
17. **Lounges and saving.** Done. A lounge (`lounge 10 21 id=bench`) is a painted chaise and standing lamp;
    pressing up beside it heals, lifts a hangover, and saves the progress and the lounge
    to `save.ron` in the data folder, which the next start loads.
    A blackout closes a slow curtain and wakes the player at the last lounge with a hangover,
    the Comedown, until they rest. The antechamber and the ballroom's alcove each have one.
    The web build does not save yet.
18. **The reachability checker.** Done. `world::reach` explores a room from where the player enters,
    playing short input scripts (walk, jump, Swoon, Lunge, wall jump, take hold, swing and let go)
    through the real controller from every place it can stand, cling or hang, and follows doors across the world.
    `cargo xtask reach` prints what each step of the progression reaches; tests check that the slice
    reaches every room and pickup, traps nobody, and keeps the ballroom's alcove for the Lure.
    The map image `cargo xtask map` is not built yet.

Then the slice's content:

19. **Nectar and the High.** Done. Guests drop golden Nectar as they fall, more from heavier ones,
    and leftovers lie about the rooms (`nectar x y`); it drifts to the player when they come near.
    The left trigger spends it: a tap heals 30 for 20 Nectar, a half-second hold starts a High for 50.
    A High lasts 14 seconds divided by one plus tolerance, which grows 0.4 with each;
    a 9-second Comedown follows. The High quickens movement by a tenth and the Comedown slows it.
    Resting lowers tolerance, never to nothing once it has started, and lifts a hangover.
    Carnations raise health by 20 and Goblets the Nectar carried by 25. The HUD has a Nectar bar.
20. **The Plunge and cracked floors.** Done. The right bumper in the air hangs for a moment,
    then dives straight down at 760 px/s. Landing on `cracked` stone shatters the whole connected patch
    and dives on through; broken tiles are kept in the progress record and stay broken.
    The reachability checker counts cracked floors as open once the Plunge is found.
    Not yet: the Plunge as an attack on guests below.
21. **Two more guests.** Done, by Codex. The spinner pirouettes across floors and hurts on contact,
    then stands dizzy; the toaster keeps its distance and throws coupes in an arc,
    which a swing destroys in flight.
22. **The rooms**: about twenty around the ballroom, ending at the stage and the cellar below it.
23. **The Dancer**, the ballroom's boss, whose fight ends with the stage floor giving way.
24. **The map.** Done. Select opens it and pauses the game: every room visited,
    drawn from its own tiles at its place on the world map, the lounges marked in gold,
    and the player a blinking dot. Rooms visited are kept in the progress record.
    `cargo xtask map` draws the whole world to an image.

## For the director

Play the ballroom with a gamepad; `cargo run --bin pixelvania` starts there.
Its done test is that it looks like the game we described, and that you sign off on the music's direction.
Climb it both ways: by the chandeliers from the dance floor to the gallery and on to the lounge,
and by the lanterns up the shaft. Then press F6 to see the progression fade, and F4 for the High.

Questions this room is meant to answer:

- **Does the progression fade stay?** At full fade the stone, the gilt and every costume go grey,
  even where a coloured lamp falls on them, and only the lamps, flames and glows keep their colour. The player fades too, since the rig is drawn into the colour buffer;
  keeping the player's colour is a small change if the fade should spare them.
- **How does the Lure feel?** Every number is in the F1 panel under *Lure*.
  The rope reels in to 96 px, swings at up to 560 px/s, and letting go multiplies the speed by 1.12.
  Once held, a rope stays held when the swing carries its line past a corner; it only needs sight to take hold.
- **Is the room bright enough?** It follows the art direction's dark silhouettes and vivid lights,
  which in a hall this size leaves stretches of near-black between the chandeliers.

The combat lab's done test still stands: a fight against the heavy guest with the Jeroboam,
once in neutral or the High and once in the Comedown (F4).

Things only you can judge, because nobody has seen or heard them in play:

- **The sounds.** Every effect was made and checked by its waveform and levels; none has been listened to.
- **The music.** The ballroom sketch decides the musical direction, which is yours to sign off.
  Listen in the combat lab, and press F4 to hear the High and the Comedown mixes.
- **The finisher.** Stagger a guest (break its poise with light hits, or a charged Jeroboam slam
  when it is not mid-swing), then hit it: it should die in a burst of pink, with slow motion.
  It works in code, but no scripted run could time it, so it has not been seen.
- **Rumble.** Written for every hit by weight, never felt on a pad.
- **The Lure's sounds.** A catch plucks a D and an A in the ballroom's key; letting go is a soft swish.

Decisions taken with a default, to confirm or change:

- **Buttons.** Attack is West (X, Square), the Toast is the left bumper, North (Y, Triangle) switches weapons,
  and the Lure is the right trigger, as the game-feel page has it.
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
