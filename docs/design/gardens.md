# The gardens

The gardens are the house's grounds under glass, reached from the bottom of the coat lift.
Their feeling is sensual and overgrown: beds gone wild in the dark,
and flowers that glow because it is always night.
They give the Lure, and their boss is the Sleeper.

The way in is a door at the bottom of the coat lift's shaft, beside the gloves that give Cling,
so the gardens open as soon as the player has dropped into the shaft.
The grand stair's garden doors stay shut; their card sends the player down, the way the coats go.

## The rooms

| Room | What happens there |
|---|---|
| The potting shed | The way in, the gardens' first lounge, and the gardener. |
| The glasshouse | A pillar in the middle holds the night-blooming flower: cling up it for the Lure, and over it to go on. Blooms hang low on either side to try it on. |
| The grotto | Under the glasshouse, through a hole in its floor past the pillar: a cave lit blue by mushrooms, with a Favour on a shelf under a bloom. Ledges climb back to the hole. |
| The bloom walk | A long drop under a trellis of blooms. The door on is high in the east wall, reached by swinging from bloom to bloom; a fall lands on the path, and steps climb back. |
| The Sleeper's bower | The arena: an open floor with blooms overhead. |
| The summerhouse | The gardens' second lounge, a Favour on a shelf, and a card. |

The player reaches the glasshouse, the Lure and the grotto with the Lunge and Cling,
and the bloom walk's far door, the bower and the summerhouse with the Lure.
The feast and the gardens can be visited in either order.
`crates/world/tests/reach.rs` checks this order against the rooms.

## The Sleeper

The Sleeper lay down in the flowers early in the night and has not got up.
He must not be woken, and everyone wakes him.
His fight teaches the Lure: the floor becomes no place to be, and a bloom is somewhere else to be.

He starts asleep, and a blow wakes him, startled, into a thrash. Then he fights in three phases.

- *Light sleep*: a thrash, flailing out both ways at once, signalled by twitching with his arms drawn in;
  then he lies down again. Asleep he is open to blows, until three of them wake him.
- *Restless*, from 60% health: the vines, signalled by his arms raised and trembling and the floor glowing.
  They come up through the whole floor for longer than a jump lasts; hang from a bloom until they sink.
- *Nightmare*, under 25%: he no longer sleeps, and the vines and thrashes come back to back.

He is a guest of his own class in the rig, in a nightgown;
`crates/game/src/sleeper.rs` is his brain and `assets/combat/boss-sleeper.ron` his frame data.
He fights to his own lullaby, which [the audio notes](audio.md#boss-themes) describe.

His lines, one per phase and one as he falls:
"Five more minutes.", "Who's there? Who is that?", "Let me sleep. Let me sleep."
and "Is it morning? Don't wake me when it's morning."
Beaten, he gives two Favours, as every boss does.

## Blooms

A bloom is a glowing flower hanging from a vine, the gardens' Lure point,
beside the ballroom's chandeliers and the Arrival's lanterns.
It is lit cyan-green, swings on its vine, and flares when the Lure catches it.
A room places one with `bloom x y`, and its vine runs up to the ceiling above.

## Props

The gardens have their own props: a raised flower bed whose flowers glow, a stone bench,
a birdbath with a bird on its rim, and a cluster of mushrooms with glowing blue caps.

## Changes elsewhere

The Lure used to be found in the ballroom's crystal gallery; its perch there now holds a Favour,
which the Swoon reaches.
The coat lift's climb was a trap: two ledges against its walls stopped a player with Cling
from climbing out of the bottom. Both are gone, and the checker now enters the shaft from below too.

## Its words

The gardener in the potting shed is new,
and so are the Host's cards in the summerhouse and on the garden doors.
`assets/story.ron` holds every line.
