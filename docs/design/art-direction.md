# Art direction

How the game looks, and how we make it look that way at a scale a small team can sustain.
The look follows one rule from [vision](vision.md): light is where the colour lives.

## The rule

Surfaces are dark, flat and low in saturation.
Light is vivid, and it does the work that detail would do in a busier style.
A wall is two or three dark tones with a one-pixel rim where light touches it.
A chandelier is a burst of warm glow that turns everything near it gold.

This rule comes from pixelfx's own quality bar, "lines and light, not fills",
and we apply it to the whole game instead of only to effects.
That makes effects and world look like one thing,
and it is also what makes the style affordable.
A dark silhouette with an automatic rim takes a fraction of the time of a fully rendered tile.

## Resolution and scale

The game renders at 640 x 360 and scales up by whole numbers.

| Display | Scale | Fit |
|---|---|---|
| 1280 x 720 | 2x | Exact |
| 1920 x 1080 | 3x | Exact |
| 2560 x 1440 | 4x | Exact |
| 3840 x 2160 | 6x | Exact |
| Steam Deck, 1280 x 800 | 2x | 40 px bars above and below |

Tiles are 16 x 16 pixels, so a screen is 40 x 22.5 tiles.
Characters are about 24 x 38 pixels, the scale pixelfx draws its effects at,
so effects fit our characters without being redrawn.
That puts the player at about a tenth of the screen's height, which leaves room to see where you are going.

We considered 480 x 270, which makes the player larger and each screen cheaper to fill.
It does not divide evenly into 1440p or the Steam Deck, so it would need non-integer scaling on common displays.
Combat weight comes from timing, reaction and sound more than from sprite size,
and enemies and bosses are free to be much larger than the player,
so the smaller player costs combat less than blurry scaling would cost everything.

The world is drawn on the pixel grid and never between pixels.
The camera still moves smoothly:
we render a frame one pixel larger than the screen on each side,
then shift the upscaled image by the camera's sub-pixel offset.
The world stays crisp, and slow camera pans do not stutter one pixel at a time.

## Three buffers

The renderer composes every frame from three buffers.
This is the part of the pipeline that makes the look possible, so it gets the most explanation.

The colour buffer holds what things look like: tiles, characters, props.
It is drawn dark and flat, from the palette's surface ramps.

The light buffer holds how lit each pixel is.
Lights from the level, glowing props and every pixelfx effect draw into it.
It multiplies the colour buffer, and it is quantised into a few bands,
so light falls off in clean pixel-art steps rather than smooth gradients.

The glow buffer holds additive light: sparks, flames, hot cores, halos.
It is added on top, after lighting, and is what makes hot things look hot.
This is the same split pixelfx uses between its `color` and `light` buffers,
which is why its effects drop into our pipeline as they are.

Notice what falls out of this.
A pixelfx hit spark draws into the light buffer as well as the glow buffer,
so every sabre hit briefly lights the walls around it.
An explosion in a dark corridor shows you the corridor.

## Automatic rim light

Artists draw environment tiles flat.
The renderer finds every solid pixel next to an empty one,
reads the light at the empty neighbour, and lights the solid edge with it.
Rims appear wherever real light is, in the light's colour, and nowhere else.

This is the single largest saving in the art budget.
It replaces hand-painted highlights on every tile in every lighting situation,
and it makes moving lights, like a swinging lantern or a passing spark, light the edges they pass.

## Palette and grading

The master palette has two families.
Surface ramps are dark and desaturated: stone, wood, fabric, skin, each in four or five steps.
Light ramps are saturated: candle gold, nectar amber, rose, absinthe green, cold cyan.
Each biome picks a few surface ramps and one or two dominant lights.

A colour grade runs last, as a 3D lookup table, and it is how the mood shows on screen.
The High pushes saturation and warmth up.
The Comedown pulls them down and cools the image.
The progression fade desaturates the colour buffer only, never the light or glow buffers,
so a faded world is grey stone under vivid lamps.

We considered storing all art as palette indices and recolouring through a palette texture.
That makes recolouring trivial, but indexed pixels cannot blend,
and pixelfx's smoke and vapour depend on real translucency.
A lookup table on true colour gives us the same mood control and keeps the blending.

## Characters

Characters are the most expensive art in a metroidvania,
because every action needs an animation and every enemy needs a set.
We build them all on pixelfx's character rig, ported to Rust.

The rig is a 3D skeleton with fixed bone lengths, solved from where the hands and feet should be,
and drawn down to pixels as a few flat tones with a single ink line, the way Dead Cells builds its sprites.
Poses are keyframes held on 12 frames per second steps, which keeps the rhythm of hand-drawn animation.
A new animation is a list of poses rather than a sheet of drawings,
and a new enemy is new proportions and new parts on the same solver.

It suits heavy combat particularly well.
pixelfx built the rig to sell hits: wind-ups, freezes on contact, follow-through,
and weapons that lag the hand so the wrist breaks and the blade whips.
The rig also shades each body part from a light direction,
which we feed from the nearest real light, so a guest walking past a candle is lit from that side.

The rig's weakness is that its stock figure is a mannequin.
Silhouette answers that: long coats, big hair, masks, oversized hats and weapons.
A costume party is the one setting where exaggerated outlines are exactly right.
[Production](production.md) explains why hand-drawn frames are not an option for this team.

## Environments

Rooms are text files of 16 x 16 tiles (see [production](production.md)),
and auto-tiling rules pick each tile's edges and corners from its neighbours,
so a room says where stone is, not which stone tile goes where.
Tiles come from material generators, such as stone, wood, velvet and gilt,
each a few lines of seeded code over a palette ramp, plus small hand-written motifs for ornament.

Most of each screen is near-black silhouette;
detail goes where the light is, because that is where the eye goes.

Backgrounds are a few parallax layers of silhouettes,
each farther layer tinted towards the biome's dominant light, like haze.
Foreground layers pass in front of the player, dark and out of focus,
and give rooms depth for almost no drawing.

## Effects

Effects come from pixelfx, ported to Rust (see [tech](tech.md)).
Its library of 244 effects is our reference and starting stock.
We recolour each one into our light ramps through its `palette` option,
and write new effects in the same style for what the game needs:
nectar bursts, the High's onset, chandelier sparks, dissolving guests.

pixelfx's authoring method is how we make them:
draft the effect as code, render a contact sheet, look at it, fix the worst thing, repeat.
Its quality bar in `docs/AUTHORING.md` is our quality bar.

## Accessibility

Screen shake, flashing and the strength of the colour grade each have a setting.
The High's flash on onset respects the flashing setting.
Important gameplay information never relies on colour alone,
which matters more than usual in a game that drains its own colour on purpose.
