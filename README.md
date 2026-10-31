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

`cargo dev` runs the game with the `dev` feature:
tuning in `assets/tuning.ron` and rooms in `assets/rooms/` reload when you save them.
Press F1 for a panel that edits the tuning live and saves it back to the file.
F2 pauses the game, and F3 steps it one gameplay tick at a time while paused.
F4 cycles the mood of combat: neutral, the High and the Comedown.
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
| Next weapon: the sabre, the Jeroboam, the candelabrum | North (Y, Triangle) | L |
| Next test room: the feel lab or the combat lab | Start | Tab |
| Back to the spawn point, and the guests back to theirs | Select | R |

Hold down while falling to fall faster, and press into a wall while falling to slide down it.
The number keys 1 to 6 set off each pixelfx effect in front of the player.

For the web build, install [trunk](https://trunkrs.dev) and run `trunk serve --release` in `web/`.

## Checking the game without playing it

`PIXELVANIA_SCREENSHOT=out.png cargo run -p game` saves a screenshot once the game has settled, then exits.
`PIXELVANIA_HOLD=right,jump,lunge` holds those inputs from the start;
jump is pressed at frame 60 and the Lunge at frame 90.
`PIXELVANIA_RECORD=path` writes every tick's input to a file, and `PIXELVANIA_REPLAY=path` plays it back.
`PIXELVANIA_PANEL=1` opens the tuning panel at start, with the `dev` feature.
`PIXELVANIA_FX=sword-slash` sets off that effect in front of the player at frame 80.
`PIXELVANIA_ROOM=arena` starts in the combat lab, and `PIXELVANIA_MOOD=high` or `comedown` in that mood.
`PIXELVANIA_HOLD` also takes `attack` (a three-hit combo), `charge` and `toast`.

## Effects and the rig

`crates/pixelfx` is a Rust port of [pixelfx](https://github.com/patch-gg/pixelfx), MIT licensed.
`cargo xtask fx <id>` renders an effect to a contact sheet and a GIF in `target/fx/`,
and `cargo xtask rig` renders the character rig's animations to `target/rig/`;
`cargo xtask rig player` renders every pose of the player's animation, one row per state.
`cargo xtask golden` renders reference frames from the JavaScript original, which needs Node and git;
the golden tests themselves run without either.
In the game, effects land on the room's floors, hide behind its walls, and light what is around them.

## Tiles

`cargo xtask tiles` renders material variants and all neighbour masks to `target/tiles/`,
and whole rooms to `target/tiles/rooms/`, with native and nearest-neighbour 2x previews.
`crates/assetgen` paints stone, wood, velvet and gilt from seeded five-colour ramps.
The game uses the same painter when a room loads or reloads.
