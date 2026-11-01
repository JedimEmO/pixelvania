# The Arrival

The Arrival is the house's front rooms, where a new game starts.
Its feeling is anticipation: warm candlelight, the party heard through the walls,
and doors that promise more than they show.
It gives the Lunge and Cling, and its boss, the Doorman, keeps the way into the rest of the house.

The grand stair leads on to the feast, at the very top of its east wall, and past the Doorman to the ballroom.
Its doors on the landings stay sealed, each with a card from the Host:
the feast's card sends the player upstairs, and until the gardens exist the ballroom keeps the Lure.

## The rooms

| Room | What happens there |
|---|---|
| The vestibule | A new game wakes on the chaise by the front doors, which stay shut until the end of the night. The first lounge. |
| The receiving line | The first guests, lined up to greet whoever comes in. A stairwell in the floor goes down to the porter's lodge. High over the east gallery, a Goblet for whoever comes back with the Swoon. |
| The porter's lodge | Luggage nobody unpacked, the porter, and a Favour a Lunge away. Landings zigzag back up to the stairwell. |
| The grand stair | The hub. Steps climb to the sealed doors of the feast and the gardens; above them the walls need Cling. |
| The coat check | The attendant, and a pair of dancing shoes on the counter that give the Lunge. A gap too wide to jump leads to the coat lift. |
| The coat lift | A shaft. The player falls in, finds long silk gloves at the bottom, and climbs out with Cling. |
| The music room | The second lounge, before the Doorman, and a pianist who knows one tune. |
| The portrait hall | Guests of other nights on the walls, a Goblet, and a drop back down to the vestibule. |
| The letter room | Invitations sorted into pigeonholes, a Favour, and a card. High on the grand stair's west wall. |
| The Doorman's hall | The boss's arena, before the inner doors. |
| The inner doors | The last hall, opening east onto the powder room and the ballroom's wing. |

The player can reach the vestibule, the receiving line, the lodge, the foot of the stair and the coat check with nothing.
The Lunge opens the coat lift and the lodge's Favour.
Cling opens the rest of the stair, and with it the music room, the portrait hall, the letter room, the Doorman and the feast.
`crates/world/tests/reach.rs` checks this order against the rooms.

## The Doorman

The Doorman has let everyone in, and has never let anyone out.
He stands before the inner doors with the guest list, a heavy guest in livery,
and his fight teaches the thing the Lunge is for: getting past a body instead of through it.

He fights in three phases, each with its own telegraph.

- *The list*: a long shove with his staff that throws the player back, signalled by the staff drawn back level.
  The knock, his staff struck on the floor as when announcing a guest, sends a line out along the floor
  both ways; jump it. It is signalled by the staff raised high.
- *Impatience*, from 60% health: a rush across the hall, shoulder down, signalled by two stamps.
  The rush is his body and too tall to jump; the Lunge passes through it, and nothing else does.
  A rush that ends in a wall staggers him.
- *Closing time*, under 25%: the moves come back to back, with shorter windups.

He is a guest of his own class in the rig, so he is hit, staggers and falls as guests do;
`crates/game/src/doorman.rs` is his brain and `assets/combat/boss-doorman.ron` his frame data.
He fights to his own processional, which [the audio notes](audio.md#boss-themes) describe.

His lines, one per phase and one as he falls:
"Name?", "You're not on the list.", "Nobody leaves. Everyone's inside." and "Go on through, then. They all do."
Beaten, he gives two Favours, as every boss does.

## Its words

The Host's first card moves from the antechamber to the vestibule: *You are expected. Do come through.*
The attendant moves from the ballroom's cloakroom to the coat check, with the same lines.
The porter and the pianist are new, and so are the cards on the sealed doors, in the letter room and at the inner doors.
`assets/story.ron` holds every line.
