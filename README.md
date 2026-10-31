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

| Action | Gamepad | Keyboard |
|---|---|---|
| Move | Left stick, d-pad | WASD, arrow keys |
| Jump, Swoon in the air | South (A, Cross) | Space |
| Lunge | East (B, Circle) | Left Shift |
| Back to the spawn point | Select | R |

Hold down while falling to fall faster, and press into a wall while falling to slide down it.

For the web build, install [trunk](https://trunkrs.dev) and run `trunk serve --release` in `web/`.

## Checking the game without playing it

`PIXELVANIA_SCREENSHOT=out.png cargo run -p game` saves a screenshot once the game has settled, then exits.
`PIXELVANIA_HOLD=right,jump,lunge` holds those inputs from the start;
jump is pressed at frame 60 and the Lunge at frame 90.
`PIXELVANIA_RECORD=path` writes every tick's input to a file, and `PIXELVANIA_REPLAY=path` plays it back.
`PIXELVANIA_PANEL=1` opens the tuning panel at start, with the `dev` feature.

## Effects and the rig

`crates/pixelfx` is a Rust port of [pixelfx](https://github.com/patch-gg/pixelfx), MIT licensed.
`cargo xtask fx <id>` renders an effect to a contact sheet and a GIF in `target/fx/`,
and `cargo xtask rig` renders the character rig's animations to `target/rig/`.
`cargo xtask golden` renders reference frames from the JavaScript original, which needs Node and git;
the golden tests themselves run without either.
