# Plan

The plan for a pixel art metroidvania about nihilistic hedonism, built in Rust on Bevy, played on a gamepad,
and made by a team of AI agents with one human director.
This page is the entry point: what to read, what we build in which order, and what is still undecided.

## The design pages

| Page | What it covers |
|---|---|
| [Vision](design/vision.md) | Theme, the feelings we aim for, world, abilities, the High and the Comedown |
| [Game feel](design/game-feel.md) | Controls, movement, camera, how we tune |
| [Combat](design/combat.md) | Weight, moves, weapons, enemies, bosses |
| [Art direction](design/art-direction.md) | Resolution, the three-buffer renderer, rim light, palette, characters, effects |
| [Audio and music](design/audio.md) | Musical direction, stems and mood mixing, the audio engine |
| [Production](design/production.md) | The team, assets as code, rooms, music, the content budget |
| [Technology](design/tech.md) | Crates, workspace, tools, rendering, the pixelfx port, platforms, testing |

## The shape of the game

Five to ten hours: seven areas and a short epilogue, about 150 rooms,
eight abilities, three weapons, about twenty-five enemy types and eight bosses.
Movement is light and fast; combat is heavy and committed.
The art style is ours to set, and the design pages set it.

## Order of work

The order follows the risks.
Feel comes first because both pillars rest on it,
and the asset tools come before content because they decide what content costs.
Each milestone ends with a test, and most tests need you with a controller or headphones.

### Foundations

The workspace, and a renderer that draws a 640 x 360 frame through the three buffers,
scales it by whole numbers and moves the camera at sub-pixel precision.
Gameplay on a fixed 120 Hz tick with buffered input from a gamepad.
Tuning files that reload while the game runs.
CI building for Linux, Windows, macOS and the web.

Done when a grey rectangle moves around a lit room on a gamepad,
on a desktop, on a Steam Deck and in a browser.

### Feel lab

The player controller with every technique in [game feel](design/game-feel.md), and the camera,
in a grey test room built from the text room format.
Headless tests pin the jump and dash metrics.

Done when you, and anyone else you can get to try it,
play the empty room for ten minutes because you want to.
This is the most important test in the plan.

### pixelfx in Rust

This runs alongside the feel lab, and it is the first large piece to hand to Codex.
Port the core and the rig, set up the golden tests against the JavaScript original,
and port six effects: hit spark, heavy landing, dust, sabre slash, hammer smash and poof.
Build `cargo xtask fx` and `cargo xtask rig`,
and connect the effects' particles to the tile grid and their light to the light buffer.

Done when a slash lights the walls around it, its sparks bounce off the real floor,
and the golden tests pass.

### Combat lab

The player rigged and animated, the sabre and the Jeroboam with their full move sets,
hit-stop, directional shake, rumble, poise, stagger, launch and wall splat,
and three enemies, one from each weight class.
Frame data in RON, the hitbox overlay and tick-by-tick stepping.

Done when a fight against the heavy enemy with the Jeroboam feels heavy in your hands,
and the same fight in a Comedown feels hollow.

### One finished room

The asset generators for tiles and backgrounds, the first music track rendered from code,
and one room of the ballroom built with them at final quality:
automatic rim light, parallax, a lit chandelier to Lure from, a costumed guest,
music stems mixed by mood, and switches for the High, the Comedown and the progression fade.

Done when the room looks like the game we described, and you have listened to the music and signed off on its direction.
This milestone also decides whether the progression fade stays.

### Vertical slice

The ballroom, since it shows the most movement and the clearest version of the theme.
Around twenty rooms, Lunge, Swoon and Lure, all three weapons,
five enemy types and the ballroom's boss,
Nectar, the High and the Comedown, lounges, the map and saving.

Done when playtesters finish it and, without being told, describe something close to the feeling we aimed for.
If they describe a different feeling, that is the most useful result we can get at this point.
The content budget is revised from how long the slice took to build and to play.

### Production

The remaining areas in the order of the night, the remaining abilities, enemies, bosses and both endings.
Then the Steam Deck checklist, a web demo, accessibility settings and localisation.

## Open questions

How to reach Astra.
It is not installed as a command, an agent or a plugin on this machine,
so its role on the team waits until we know what it is and how to call it.

Whether the progression fade stays.
It is the boldest idea in the design and the most likely to make the game feel dull.
Decided by the one finished room.

The title.
Some candidates in the spirit of the game: *Small Hours*, *Last Call*, *Afterglow*.
Each needs a search for existing games before it is used.
