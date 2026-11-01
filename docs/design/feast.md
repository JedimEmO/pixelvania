# The feast

The feast is the house's dining rooms, up the grand stair, and the first area past the Arrival.
Its feeling is abundance, then too much: tables laid for more than will ever sit,
kitchens that never finish, and at the end of the table a guest who cannot stop.
It gives the Jeroboam and the Swoon, and its boss is the Glutton.

The way in is at the very top of the grand stair's east wall, a cling past the Doorman's door,
so the feast opens with Cling, as the Doorman does, and the player chooses which to meet first.
The feast's doors on the landing stay shut; their card sends the player upstairs.

## The rooms

| Room | What happens there |
|---|---|
| The service stair | The servants' way up. Landings zigzag from the grand stair's door to the banquet hall's, high on the east. |
| The banquet hall | The great room: a high table to climb over, a gallery under the west door, and landings up the east side to the kitchens and the long table above. A Favour on a shelf over the table, for a good Lunge. |
| The supper room | The feast's first lounge, off the banquet hall's floor, and the gourmand. |
| The kitchens | Ovens on the floor, cooks who fight, shelves zigzagging between the door and the floor, and a Goblet on a high shelf by the east wall. |
| The larder | A cold dead end off the kitchens, with a card from the Host. |
| The long table | Above the banquet hall, through a hole in its ceiling: the table the whole feast is laid along, and the sideboard's lounge before the Glutton. |
| The pastry room | Sugar towers and a Favour a Lunge from the highest landing. |
| The Glutton's hall | The arena. The Jeroboam stands on the table at the back. |
| The afters | Guests asleep where they fell, the Swoon by the fainting couch, and a Favour hung too high for anything but the Swoon. |

The player reaches every room and the Jeroboam with the Lunge and Cling.
The Swoon is found in the afters, after the Glutton, and its Favour there needs it.
`crates/world/tests/reach.rs` checks this order against the rooms.

## Gating with the Swoon

The Swoon is a second jump in the air, about eight tiles of height where a jump gives five.
Two other moves reach far and have to be ruled out for a Swoon gate to hold.
Cling grabs any wall, including the end of a floating ledge,
so a ledge whose underside a jump can touch is a Cling climb, not a Swoon one.
The Lunge carries its speed a long way through the air,
so from high on a wall a player can lunge across most of a room as they fall.

The afters' Favour is therefore a ledge eight tiles above the floor,
far from both walls, under a ceiling low enough that a fall from a wall cannot carry to it.
The balcony's ledges up to the upper gallery follow the same rule.

## Its words

The gourmand in the supper room and a guest lying down in the afters are new,
and so are the Host's cards in the larder and on the feast's sealed doors.
`assets/story.ron` holds every line.

The Swoon used to be found on the ballroom's balcony and the Jeroboam in the champagne cellar.
Those places now hold a Goblet and a Favour, and the balcony's west end has planters a player
with only Cling can climb, so the ballroom's wing never needs the Swoon to leave.
