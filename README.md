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

A new game starts in the Arrival, the house's front rooms, where the Lunge and Cling are found
and the Doorman keeps the inner doors. Up the grand stair is the feast, where the Glutton keeps the Swoon.
Past the Doorman is the vertical slice: twenty rooms of the ballroom,
with the Dancer waiting on the stage and the cellar beneath it where the slice ends.
`cargo xtask map` draws them all; Select shows the ones you have visited.
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
| Tap to drink a Cordial and heal, hold to spend Nectar on a High | Left trigger (LT, L2) | N |
| Plunge: in the air, dive straight down; breaks cracked floors | Right bumper (RB, R1) | M |
| Next weapon: the sabre, the Jeroboam, the candelabrum | North (Y, Triangle) | L |
| Next room, for testing: each room of the world, the feel lab, the combat lab | Start | Tab |
| The map of the rooms visited; pauses the game. In a test room, back to the spawn point | Select | V |
| Back to the spawn point, and the guests back to theirs | | R |

Hold down while falling to fall faster, and press into a wall while falling to slide down it.
Press up beside a lounge, a velvet chaise under a standing lamp, to rest:
it heals, fills your three Cordials, saves, and is where you wake after a blackout.
Resting opens the lounge's menu, where each Favour you have found buys a level
in Vitality, Composure, Strength or Appetite. Favours hide in the rooms, and the Dancer gives two.
The HUD shows health, poise, a pip for each Cordial, the Nectar bar marked at the cost of a High,
and the Favours you carry; a line under it says what the left trigger would do.
The save is `save.ron` in your data folder (`~/.local/share/pixelvania` on Linux),
or the browser's local storage in the web build, and the game starts from it;
delete it, or set `PIXELVANIA_FRESH=1`, for a new game.
The slice ends in the cellar under the stage, the first time you land there with the Plunge:
a card shows the night so far, and lets you keep playing or start a new night.
On the Lure's rope, push the stick the way you want to swing, and up or down to climb or lower.
Brackets mark the light the Lure would take hold of.
A prompt such as `[UP] Talk` shows over anything that can be used, and the prompts, banners and menus
name the buttons of the device last pressed, keyboard or gamepad.
The number keys 1 to 6 set off each pixelfx effect in front of the player.

For the web build, install [trunk](https://trunkrs.dev) and run `trunk serve --release` in `web/`.
Every push to `master` builds it and publishes it to https://jedimemo.github.io/pixelvania/,
by [the web workflow](.github/workflows/web.yml).

## Checking the game without playing it

`PIXELVANIA_SCREENSHOT=out.png cargo run -p game` saves a screenshot once the game has settled, then exits.
`PIXELVANIA_HOLD=right,jump,lunge` holds those inputs from the start;
jump is pressed at frame 60 and the Lunge at frame 90.
`PIXELVANIA_RECORD=path` writes every tick's input to a file, and `PIXELVANIA_REPLAY=path` plays it back.
`PIXELVANIA_PANEL=1` opens the tuning panel at start, with the `dev` feature.
`PIXELVANIA_FX=sword-slash` sets off that effect in front of the player at frame 80.
Every switch that picks the start, below, also ignores the save.
`PIXELVANIA_ROOM=arena` starts in the combat lab (or `lab`, `ballroom`),
`PIXELVANIA_MOOD=high` or `comedown` in that mood, `PIXELVANIA_FADE=0.5` with that much progression fade,
`PIXELVANIA_SPAWN=34,13` with the player at that tile,
and `PIXELVANIA_PROGRESS=new` or `all` with nothing found or everything,
instead of what the vertical slice assumes: the Lunge, Cling, the Swoon, the Lure, the sabre and the Jeroboam.
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
`cargo xtask map` draws every room of the world at its place on the map to `target/map.png`.
`cargo xtask reach` walks the world with each step of the progression's abilities, using the real movement,
and lists the rooms, pickups and lounges it reaches,
and any way in that leads neither back to the start nor on to a new ability.
`cargo xtask hanging` renders the chandelier and the lantern, at rest, swung and flared, to `target/hanging/`.
In the game, effects land on the room's floors, hide behind its walls, and light what is around them.

## Tiles

`cargo xtask tiles` renders material variants and all neighbour masks to `target/tiles/`,
and whole rooms to `target/tiles/rooms/`, with native and nearest-neighbour 2x previews.
`crates/assetgen` paints stone, wood, velvet and gilt from seeded five-colour ramps.
The game uses the same painter when a room loads or reloads.

## Backgrounds

`background = ballroom` in a room's `[room]` section selects generated parallax scenery:
`ballroom`, `cellars`, `backstage` or `foyer`. A colour after the name tints its distance,
as in `background = ballroom 345a32`; without one it is a dim wine-rose.
`cargo xtask backgrounds` writes each set's strips and three camera offsets to `target/backgrounds/<set>/`,
with native and nearest-neighbour 2x previews.
The game generates the same seeded layers on room load.

## Ballroom music

`cargo xtask music` renders five deterministic WAV stems to `assets/music/ballroom/`.
The stems are not committed, being 54 MB of WAV; run it once after cloning, or the game plays without music.
It also renders the bosses' themes beside them,
and writes full mixes, each phase's mix, waveforms, loop-seam views and spectrograms to `target/music/`.
The sketch is 64 seconds at 120 BPM in D minor;
[the audio notes](docs/design/audio.md#ballroom-sketch) describe its form and instruments.
`music = ballroom` selects it in a room's `[room]` section, including the arena,
and `music = ballroom muffled` hears it through a wall, as the Arrival does.
On native builds, F4 changes the stem mix, filter and reverb over 1.5 seconds.
Music fades out in other rooms and stays silent without an audio device.
The web build plays the stems as OGG Vorbis: the web workflow renders them and encodes them with ffmpeg.
To hear music in a local web build, put `.ogg` files of the stems beside the WAVs first.
