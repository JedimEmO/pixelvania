# pixelvania

A pixel art metroidvania about nihilistic hedonism,
made in Rust on Bevy and designed for a gamepad.
Working title.

The project is early in development.
Start with [the plan](docs/plan.md), which links to the design pages.

## Running

```sh
cargo dev
```

The game starts in the antechamber, whose door leads into the ballroom, the first room at final quality:
a dance floor, an orchestra pit, a lantern-lit shaft, a gilt gallery and a lounge,
joined by chandeliers to swing from with the Lure.
The rooms of the game are listed in `assets/world.ron` and joined by doors in their room files.
Start or Tab steps through every room, then the feel lab and the combat lab.

`cargo dev` runs the game with the `dev` feature:
tuning in `assets/tuning.ron` and rooms in `assets/rooms/` reload when you save them.
Press F1 for a panel that edits the tuning live and saves it back to the file.
F2 pauses the game, and F3 steps it one gameplay tick at a time while paused.
F4 cycles the mood of combat: neutral, the High and the Comedown.
F6 steps the progression fade, which drains the colour from surfaces but not from light.
F5 shows hurtboxes, active hitboxes, and health and poise bars,
and the keys 7, 8 and 9 spawn a light, medium or heavy guest in front of the player.
Weapon frame data in `assets/combat/` reloads while the game runs, with the `dev` feature.

| Action | Gamepad | Keyboard |
|---|---|---|
| Move | Left stick, d-pad | WASD, arrow keys |
| Jump, Swoon in the air | South (A, Cross) | Space |
| Lunge | East (B, Circle) | Left Shift |
| Attack; hold to charge, with up to rise, with down in the air to pogo | West (X, Square) | J |
| Toast, the parry | Left bumper (LB, L1) | K |
| Lure: take hold of the light the stick aims at; hold to swing, release to let go | Right trigger (RT, R2) | I |
| Next weapon: the sabre, the Jeroboam, the candelabrum | North (Y, Triangle) | L |
| Next room, for testing: each room of the world, the feel lab, the combat lab | Start | Tab |
| Back to the spawn point, and the guests back to theirs | Select | R |

Hold down while falling to fall faster, and press into a wall while falling to slide down it.
On the Lure's rope, push the stick the way you want to swing, and up or down to climb or lower.
A diamond marks the light the Lure would take hold of.
The number keys 1 to 6 set off each pixelfx effect in front of the player.

For the web build, install [trunk](https://trunkrs.dev) and run `trunk serve --release` in `web/`.

## Checking the game without playing it

`PIXELVANIA_SCREENSHOT=out.png cargo run -p game` saves a screenshot once the game has settled, then exits.
`PIXELVANIA_HOLD=right,jump,lunge` holds those inputs from the start;
jump is pressed at frame 60 and the Lunge at frame 90.
`PIXELVANIA_RECORD=path` writes every tick's input to a file, and `PIXELVANIA_REPLAY=path` plays it back.
`PIXELVANIA_PANEL=1` opens the tuning panel at start, with the `dev` feature.
`PIXELVANIA_FX=sword-slash` sets off that effect in front of the player at frame 80.
`PIXELVANIA_ROOM=arena` starts in the combat lab (or `lab`, `ballroom`),
`PIXELVANIA_MOOD=high` or `comedown` in that mood, `PIXELVANIA_FADE=0.5` with that much progression fade,
and `PIXELVANIA_SPAWN=34,13` with the player at that tile.
`PIXELVANIA_HOLD` also takes `attack` (a three-hit combo), `charge`, `toast`, `finish`,
`jeroboam` and `candelabrum`, and `PIXELVANIA_GUEST=heavy` spawns a guest in front of the player.

## Effects and the rig

`crates/pixelfx` is a Rust port of [pixelfx](https://github.com/patch-gg/pixelfx), MIT licensed.
`cargo xtask fx <id>` renders an effect to a contact sheet and a GIF in `target/fx/`,
and `cargo xtask rig` renders the character rig's animations to `target/rig/`,
the costumed guests among them;
`cargo xtask rig player` renders every pose of the player's animation, one row per state.
`cargo xtask golden` renders reference frames from the JavaScript original, which needs Node and git;
the golden tests themselves run without either.
`cargo xtask hanging` renders the chandelier and the lantern, at rest, swung and flared, to `target/hanging/`.
In the game, effects land on the room's floors, hide behind its walls, and light what is around them.

## Tiles

`cargo xtask tiles` renders material variants and all neighbour masks to `target/tiles/`,
and whole rooms to `target/tiles/rooms/`, with native and nearest-neighbour 2x previews.
`crates/assetgen` paints stone, wood, velvet and gilt from seeded five-colour ramps.
The game uses the same painter when a room loads or reloads.

## Ballroom backgrounds

`background = ballroom` in a room's `[room]` section selects generated parallax scenery.
`cargo xtask backgrounds` writes each strip and three camera offsets to `target/backgrounds/`,
with native and nearest-neighbour 2x previews.
The game generates the same seeded layers on room load, with dim wine-rose distant light.

## Ballroom music

`cargo xtask music` renders five deterministic WAV stems to `assets/music/ballroom/`.
The stems are not committed, being 54 MB of WAV; run it once after cloning, or the game plays without music.
It also writes a full mix, waveforms, loop-seam views and spectrograms to `target/music/`.
The sketch is 64 seconds at 120 BPM in D minor;
[the audio notes](docs/design/audio.md#ballroom-sketch) describe its form and instruments.
`music = ballroom` selects it in a room's `[room]` section, including the arena.
On native builds, F4 changes the stem mix, filter and reverb over 1.5 seconds.
Music fades out in other rooms and stays silent without an audio device;
wasm skips music playback.
