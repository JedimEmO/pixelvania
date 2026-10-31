# pixelfx

CPU pixel effects and a character rig for the game and its offline tools.
The crate has no runtime dependencies and no Bevy types.

```rust
use pixelfx::{Effect, EffectId, Frame, Options};

let mut effect = Effect::new(EffectId::HitSpark, 4, Options::default());
let mut frame = Frame::new(136, 80);
effect.update(1.0 / 30.0, &|_x| 0.0);
effect.draw(&mut frame, [44.0, 64.0]);
```

`Frame::color` and `Frame::light` are separate RGBA buffers.
Color contains opaque pixels, ordered dither and translucent smoke;
light contains additive RGB with alpha 255 wherever a light write occurred.
Clear both buffers before drawing the next game frame.
`draw` adds to the destination, so several effects can share a frame.

## Effects and time

`EffectId::ALL` lists hit spark, heavy landing, sword slash, hammer smash, poof and dust.
`metadata()` supplies preview size, origin, duration and the stock seed.
`Options` overrides palettes, spark and star colors, stone color,
height, aim, slash radius, mirroring, rotation and light strength.
Palette ramps are resampled to the effect's expected length.
An empty palette selects the stock colors.
`DustVariant` selects `Run`, `Jump` or `Land`.

`update(dt, ground)` subdivides each call into equal steps of at most 1/120 second,
matching the original `createFx` wrapper.
Pass the same sequence of updates when exact replay matters;
variable remainder steps can change an Euler simulation.
`seek(time, ground)` rebuilds from the seed and replays at 30 fps,
including a final remainder, for offline rendering and backwards scrubbing.
The ground query must stay stable during a seek.
`stop()` fades over 0.3 seconds; `finished()` also becomes true at the stock duration.

Coordinates are art pixels with positive y down.
A `Ground` implementation, or any `Fn(f64) -> f64`, returns local floor y for local x.
The game translates that query to its tile grid, including placement and mirroring.
The callback affects spark, grit and rigid-debris collisions.
Decorative dents and floor rings remain at the effect's local y = 0.

## Rig and acts

`Rig::render(&Pose)` solves the skeleton and returns local pixels.
Draw them through a `Painter`, with the feet at its origin.
`Rig::skeleton` exposes solved joints; `shortfall` reports unreachable hands.
`Measurements`, body palettes and light direction can be changed per character.
The stock body parts and eleven weapons use the original depth and shading rules.
`Weapon::Bottle`, the game's Jeroboam, is our addition and has no counterpart in the original.
The solver uses f64; depth and tone buffers use f32, as in the original typed arrays.
There is no quantized pose cache, so rendering does not depend on earlier calls.

`act::timing(id)` exposes the three stock act tracks,
including negative wind-up times, recovery, targets and hit events.
`pose_at` holds limbs at 12 fps while interpolating world placement smoothly.
`ActClock` pauses effect time for hit-stop and emits each hit once.
The game owns camera shake and target reactions, using the event's timing and amplitude.
Preview-only straw targets, after-images and effects carried across body pixels
from JavaScript's `createAct` are outside this port.
`Actor::stock` provides both stock sprites and their masks.

## Verification

`cargo test -p pixelfx` reads committed PNG references without Node.
Each color and light buffer permits at most 0.5% differing pixels,
and reports its fraction with `-- --nocapture`.
The five stock effects each have six sampled frames;
the rig has five poses, including a turn and raised weapons.
Dust has fixed snapshots covering all 25 preview frames of each variant.
Additional tests cover RNG words, ground collisions, palette overrides,
layer composition, seeking, hit-stop and bone lengths.

`cargo xtask golden` regenerates references from the pinned JavaScript commit
`a84159688af3ca37a4be05a0d422b94451b2e24e`.
It clones into `target/pixelfx-js/` when absent.
Set `PIXELFX_JS` to reuse a clean checkout at that exact commit.
Only regeneration needs Node and Git; an initial clone also needs network access.
References contain raw buffers without actors, background, compositing or edge fade.
`tests/golden/PROVENANCE.txt` records the source and rendering conventions.

The port includes the primitives and particle kinds used by these effects.
Unselected effects, text rendering, chains, scene backdrops and unused simulation kinds
are not included.
The upstream [license](LICENSE), [notice](NOTICE.md) and per-file credits are retained.
