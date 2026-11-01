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
| The bloom walk | A long drop under a trellis of blooms. The door on is high in the east wall, reached by swinging from bloom to bloom; a fall lands on the path, and steps climb back. |
| The Sleeper's bower | The arena: an open floor with blooms overhead. |
| The summerhouse | The gardens' second lounge, a Favour on a shelf, and a card. |

The player reaches the glasshouse and the Lure with the Lunge and Cling,
and the bloom walk's far door, the bower and the summerhouse with the Lure.
The feast and the gardens can be visited in either order.
`crates/world/tests/reach.rs` checks this order against the rooms.

## Blooms

A bloom is a glowing flower hanging from a vine, the gardens' Lure point,
beside the ballroom's chandeliers and the Arrival's lanterns.
It is lit cyan-green, swings on its vine, and flares when the Lure catches it.
A room places one with `bloom x y`, and its vine runs up to the ceiling above.

## Changes elsewhere

The Lure used to be found in the ballroom's crystal gallery; its perch there now holds a Favour,
which the Swoon reaches.
The coat lift's climb was a trap: two ledges against its walls stopped a player with Cling
from climbing out of the bottom. Both are gone, and the checker now enters the shaft from below too.

## Its words

The gardener in the potting shed is new,
and so are the Host's cards in the summerhouse and on the garden doors.
`assets/story.ron` holds every line.
