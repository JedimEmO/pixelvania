# Production

How the game gets made by a team of AI agents with one human director,
and how every asset is produced in a form that team can work with.

## The team

| Member | Role |
|---|---|
| You | Director. The final judge of feel, look and sound; playtester; the one who holds a controller and listens |
| Claude | Lead. Design, architecture, integration, reviews, keeping these pages true |
| Codex | Parallel implementation of well-specified pieces, and a second opinion on hard bugs and designs |
| Astra | To be decided once we know how to reach it |

Work is split so that each agent gets a self-contained task with a clear way to check it:
a pixelfx effect to port with its golden images, an enemy with its frame data, a room with its ability gates.
Claude reviews everything before it lands.

## What the agents cannot do

Agents can read code and look at images, but they cannot hear audio or feel a controller.
That shapes the whole pipeline.

Art can be checked by agents directly, by rendering it to an image and looking at it.
Music and sound can only be checked by agents indirectly:
by music theory, loudness measurements and spectrograms.
Whether a track is good, and whether a jump feels right, is your call,
and the plan builds in regular points where you play and listen.

## Assets as code

An agent cannot paint in Aseprite or drag tiles around in an editor.
So every asset in the game is either code or a text file,
rendered by a tool, and looked at before it is used.
pixelfx already works this way for effects; we extend the idea to everything else.

The loop is the same for every kind of asset.
Write it as code or text, render it, look at the render, fix the worst thing, render again.
Each kind of asset has a tool that renders it to something an agent can look at.

| Asset | Written as | Rendered to |
|---|---|---|
| Asset | Written as | Rendered by | To |
|---|---|---|---|
| Effects | pixelfx effect code in Rust | `cargo xtask fx` | Contact sheets and GIFs |
| Characters and enemies | Rig proportions, parts and poses in RON | `cargo xtask rig` | Contact sheets of every animation |
| Tiles and props | Material generators and small pixel grids written as text | `cargo xtask tiles` | Atlases and sample rooms |
| Backgrounds | Silhouette generators with seeds | `cargo xtask backgrounds` | Parallax previews |
| Rooms and the world | Text rooms and a world file | `cargo xtask map` | A full map image and a reachability report |
| Music | Patterns and instruments as code | `cargo xtask music` | Stems, spectrograms, loudness reports |
| Sound effects | Synthesis recipes | `cargo xtask sfx` | Audio files and waveform images |

Generated assets are rebuilt by `xtask` and checked into the repository,
so the game itself never runs the generators.

## Characters

Characters are built on pixelfx's rig, ported to Rust and generalised.
A character is a set of proportions, a list of parts drawn on the bones (a coat, a mask, a crown of grapes),
a palette, and a library of poses.
An animation is a list of timed poses, the same format pixelfx's acts use.

Hand-drawn frames are ruled out for this team; the rig is the pipeline.
The rig's weakness is that its figures can read as mannequins.
We answer that with strong silhouettes from the parts:
long coats, big hair, masks, oversized hats and weapons,
which is also what a costume party looks like.

## Rooms

Rooms are text files: a grid of characters for the tiles,
and a short list of entities with positions, such as enemies, lights, chandeliers and doors.
Text rooms are easy for agents to write and easy for you to read and edit in any editor.
This replaces LDtk, which is built around a mouse.

A world file connects the rooms and records which abilities each connection needs.
`cargo xtask map` draws the whole world as one image.
A reachability checker walks the world with each set of abilities in turn,
using the real movement metrics from the headless tests,
and reports every room you can reach, every soft lock, and every way to break the intended order.
A sequence break the design allows is listed in the world file, so the checker knows it is on purpose.

## Music

Music is composed as code.
Patterns are written in a compact text notation and rendered offline into stems.
Synthesised instruments use `fundsp`;
acoustic instruments use `rustysynth` with a permissively licensed SoundFont.

The genres in [audio](audio.md) suit this well.
Italo disco and house are built from drum machines and synthesisers, and those are exactly what synthesis does best.
FM electric piano, vibraphone and analogue pads synthesise convincingly.
A breathy saxophone or a real choir does not,
so those parts move to SoundFont instruments or to synthesised substitutes.

Each rendered track comes with a report: loudness, spectrum, key and tempo.
The report catches technical mistakes; you decide whether it sounds good.

## Content budget

For a game of five to ten hours, a first estimate:

| Content | Amount |
|---|---|
| Areas | 7, plus the dawn epilogue |
| Rooms | About 150 |
| Abilities | 8 |
| Weapons | 3 |
| Enemy types | About 25 |
| Bosses | 8, including the Host |
| Music | About 10 tracks, each with 4 to 5 stems |
| Effects | About 60, half ported from pixelfx and half new |

We revise this after the vertical slice, using how long its rooms take to play and to build.
