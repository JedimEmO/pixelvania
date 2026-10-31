# Technology

The engine, the crates, how the code is laid out, and how the pixelfx effects get into a Rust game.
Versions are as of 2026-10-31.

## Stack

The game is Rust on Bevy 0.20.

| Need | Choice | Version |
|---|---|---|
| Engine | `bevy` | 0.20.1 |
| Input | `bevy_enhanced_input` | 0.27 |
| Levels | Our own text room format | |
| Audio | `kira`, with our own thin layer | 0.12 |
| Settings, tuning, saves | `serde` and `ron`, saves under `directories` | 0.12, 6.0 |
| Debug panels, dev builds only | `bevy_egui`, `bevy-inspector-egui` | 0.43, 0.38 |
| Effects and character rig | pixelfx, ported to Rust as our own crate | |
| Music and sound rendering, offline only | `fundsp`, `rustysynth`, `hound` | 0.23, 1.3, 3.5 |

Every third-party Bevy crate is one more thing a Bevy upgrade waits for, so the list is short on purpose.
Each is added with `default-features = false` and only the features we use.

`bevy_enhanced_input` won over `leafwing-input-manager` because its input contexts map cleanly onto game states
(playing, map, menu), and its modifiers and conditions handle dead zones, holds and taps
without code of our own.
Both support Bevy 0.20.

Rooms are text files rather than LDtk levels,
because LDtk is an editor built around a mouse and our team works in text.
[Production](production.md) describes the format and the tools around it.
It also removes the one level-loading crate a Bevy upgrade would have waited for.

The music and sound crates run only in the offline asset tools.
The game ships rendered audio files and never runs a synthesiser.

There is no physics engine.
[Game feel](game-feel.md) explains why the player and enemies use our own kinematic controller.
pixelfx brings its own particle and debris simulation for effects.

## Workspace

```text
crates/
  game/       the binary and the game's Bevy plugins
  pixelfx/    the pixelfx port: frames, painter, simulation, rig, effects; no Bevy dependency
  world/      the room format and gameplay simulation; no Bevy dependency
  anim/       character animation: from how a body moves to the rig pose it holds; no Bevy dependency
  assetgen/   seeded tiles, parallax silhouettes and room auto-tiling; no Bevy dependency
  xtask/      the command line for everything that is not the game
```

Game code lives in `game` as Bevy plugins, one per concern:
render, backdrop, input, player, camera, rooms, combat, enemies, mood, audio, effects, save and ui.
We split a plugin into its own crate only when compile times or reuse ask for it.

`pixelfx`, `world`, `anim` and `assetgen` have no Bevy dependency on purpose.
Their tests run in milliseconds, the tools use them without starting an engine,
and a Bevy upgrade does not touch them.

`xtask` is the one entry point to every tool, and each one renders something an agent can look at.

| Command | Does |
|---|---|
| `cargo xtask fx <id>` | Renders an effect to a contact sheet and a GIF |
| `cargo xtask rig <character>` | Renders every animation of a character to contact sheets |
| `cargo xtask map` | Draws the whole world, and runs the reachability checker |
| `cargo xtask tiles` | Renders material sheets and all rooms to `target/tiles/`, at 1x and 2x |
| `cargo xtask backgrounds` | Renders ballroom strips and three parallax offsets to `target/backgrounds/`, at 1x and 2x |
| `cargo xtask music <track>`, `sfx` | Renders audio, with loudness and spectrum reports |
| `cargo xtask golden` | Renders reference frames from the JavaScript pixelfx |

The `mood` plugin owns one resource that the others read:
the current High, Comedown, tolerance and progression fade.
The renderer reads it for the colour grade, the audio layer for the mix, the player for its speeds.
One state driving every sense is how the game keeps them saying the same thing.

Tile materials use five dark sRGB tones and eight seeded variants each.
The painter checks all eight neighbours for edges and concave corners.
Empty cells stay transparent, and solid cells stay opaque for the renderer's rim light.
Unknown materials use a magenta checker.

The game paints rooms on load and reload through `assetgen::rasterize`.
This is an exception to the offline asset plan in [production](production.md),
so text-room edits remain visible without a separate bake step.
`cargo xtask tiles` uses the same painter and adds flat dim light for previews.

## Time

Gameplay runs in Bevy's `FixedUpdate` at 120 ticks per second.
Rendering runs at the display's rate and interpolates positions between ticks,
then rounds them to the pixel grid.

Input is read every frame and buffered for the next tick,
so a press shorter than a tick still registers.
Whether `bevy_enhanced_input` can do this for us in a fixed schedule,
or whether we buffer its action events ourselves, is the first thing to check in the prototype.

## Rendering

The world renders at 640 x 360, plus one pixel on each side for the sub-pixel camera,
into colour, light and glow targets, plus separate background and foreground targets.
[Art direction](art-direction.md) describes what each one holds.

A composite pass combines them:
colour multiplied by banded light, rim lighting on solid edges, glow added on top,
the mood's colour grade from a 3D lookup table, and a fine dither.
An upscale pass then scales the result to the window by a whole number and applies the sub-pixel offset.

Rooms opt into the ballroom with `background = ballroom` under `[room]`.
The backdrop plugin generates four 960 x 400 RGBA strips on load and reload,
using a fixed seed and a configurable dominant light in `assetgen::backgrounds::ballroom`.
This shares the tile painter's runtime exception to the offline asset plan;
room edits need no bake step, and the preview uses the same generator.

Windows, the gallery and nearer pillars move at 0.12, 0.32 and 0.62 times camera travel.
A sparse foreground curtain and palm move at 1.18 times camera travel.
Three copies of each strip repeat horizontally, with whole-pixel placement
and bounded vertical movement inside the extra rows.

The scenery's small sRGB palettes include their dim distant lighting.
The composite reads the background only where the colour buffer is empty,
then blends the foreground over the lit world and glow.
Neither scenery target contributes to the colour buffer's alpha solid mask,
so silhouettes receive neither room lighting nor automatic rims.
Rooms without scenery retain the original lit background colour.
The cost is two extra 642 x 362 render targets and camera passes.

Both passes are plain fragment shaders with no compute,
because the web build runs on WebGL2.

## pixelfx in Rust

pixelfx is a JavaScript library.
Every effect is a short program that draws into two pixel buffers, `color` and `light`,
on top of a small core: a painter, a particle and debris simulation, a seeded random number generator,
colour ramps, and the character rig.
Effects ship as code, not images, and there is no dependency beyond that core.

We had three ways to use it from Rust.

| Option | Verdict |
|---|---|
| Export sprite sheets with its own tools | Use first, for prototyping. Loses per-instance recolouring, seeds and light |
| Embed a JavaScript engine | Rejected. Adds a runtime to every platform for code that ports directly |
| Port the core and the effects we use to Rust | Chosen |

We port because the port gives us things a sprite sheet cannot.
Each effect's light buffer feeds our light buffer, so effects light the world.
Sparks and debris collide with the real level:
pixelfx's particles take a `ground` function, and ours asks the tile grid.
Every instance gets its own seed and palette, so no two hits look the same.
Hit timing from pixelfx acts drives our hit-stop directly.

The port is smaller than the library suggests.
The parts of the core we need are about 1,500 lines of JavaScript.
The 244 effects come to about 41,000 lines, but we port only the ones we use,
on demand, and recolour them on the way in.

The port is checked against the original.
`cargo xtask golden` runs pixelfx's JavaScript in Node to render reference frames,
and golden tests compare them with the Rust output at the same time, seed and options.
JavaScript numbers and Rust's `f64` are the same format,
but trigonometric functions can differ in the last bit,
so the tests allow a small fraction of differing pixels rather than demanding an exact match.

At runtime, live effects draw into a CPU frame the size of the screen.
The frame is uploaded as two textures each frame something is playing,
and the composite pass treats them as part of the colour and light buffers.

`cargo xtask fx` renders our Rust effects to contact sheets and GIFs,
so new effects are made with the same draw, look and refine loop pixelfx uses.
The rig gets the same treatment, generalised so that characters other than the stock figure can be built on it.

pixelfx is MIT licensed.
Its notice credits Hugo Duprez's Sprite Fusion effects and Hollow Crown,
and our port keeps that notice and the per-file credits.

## Platforms

| Platform | Status |
|---|---|
| Windows, Linux, macOS | First-class |
| Steam Deck | First-class; tested on the device, 1280 x 800, controller only |
| Web, WebAssembly on WebGL2 | Supported for demos and playtests; no rumble, audio starts on first input |
| Consoles | Not planned yet. Bevy has no public console support, so a port would need a partner. We avoid choices that would block one |
| Phones | Not planned. A gamepad-first metroidvania is a poor fit for touch |

## Testing

The fixed tick makes gameplay deterministic, which makes it testable.

Headless tests run the gameplay plugins without a window
and check level-design metrics: jump height and distance, dash length, coyote time.
Recorded input from playtests replays against new builds to catch changes in feel.
Golden image tests keep the pixelfx port faithful.
The reachability checker runs on every change to the world,
so a room edit that soft-locks the game fails the build.

CI builds for Linux, Windows, macOS and `wasm32`, and runs `clippy` and the tests on Linux.

## Development builds

Development builds use Bevy's dynamic linking and asset hot reloading.
Tuning files, levels and shaders reload while the game runs.
The debug panels and the inspector exist only behind a `dev` feature,
so they never ship.
