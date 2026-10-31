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
Move with the left stick or d-pad and jump with the south button (A, Cross);
on a keyboard, WASD or the arrow keys and Space.

For the web build, install [trunk](https://trunkrs.dev) and run `trunk serve --release` in `web/`.

## Checking the game without playing it

`PIXELVANIA_SCREENSHOT=out.png cargo run -p game` saves a screenshot once the game has settled, then exits.
`PIXELVANIA_HOLD=right,jump` holds those inputs from the start.
