# xtask

Offline effect previews, rig pose sheets and JavaScript reference generation.
Run commands from the workspace root.

```sh
cargo xtask fx impacts/hit-spark
cargo xtask fx impacts/heavy-landing
cargo xtask fx melee/sword-slash
cargo xtask fx melee/hammer-smash
cargo xtask fx spawn/poof
cargo xtask fx dust
cargo xtask rig
```

`fx` accepts `--seed N` and `--out DIR`.
It writes a PNG contact sheet at 2× scale and a looping GIF at 3× scale,
both with nearest-neighbour scaling.
The default output directory is `target/fx/`.
Frames run left to right, top to bottom, at evenly spaced samples of the effect.
Effects with an actor show the stock standing sprite between their back and front layers.

`rig [stock]` writes stance, turn and all three effect-act sheets to `target/rig/`.
Other character names return an error until a character definition is added.

```sh
cargo xtask golden
```

`golden` checks out the pinned upstream source in `target/pixelfx-js/`,
or uses the checkout named by `PIXELFX_JS`.
It requires Node and Git, and refuses a different commit or modified source.
See the [pixelfx README](../pixelfx/README.md) for reference-frame conventions.
