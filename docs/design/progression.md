# Progression

How the player moves through the game:
which ability is found where, in what order the areas open,
what else the player gains on the way, and what it costs them.
This page is a proposal; the choices still waiting for the director are listed at the end.

## What grows and what fades

Four things change as the game goes on, and they pull against each other on purpose.

The player gains reach. Abilities open the map, weapons open new ways to fight,
and found upgrades raise health and the Nectar a High can draw on.
Every gain makes the player stronger and the palace easier to cross.

The world loses colour. Each ability found takes a share of the colour from lit surfaces,
and light never fades, so the late game is grey stone under vivid lamps.
Tolerance grows with every High and shortens the next one.

So the further the player gets, the more they can do and the less it feels like.
The High is the only way back to the colour they remember, and it works a little less each time.
That is the theme as a curve, and every choice below serves it.

## The shape of the palace

The palace is a ring of areas around a sealed centre, like the rooms of Poe's masque,
each lit in its own colour.
The Arrival is the way in and the hub: every area opens off it or off a neighbour,
and its grand stair is the place the player keeps passing through.

| Area | Feeling | Its light | Ability found | Boss |
|---|---|---|---|---|
| The Arrival | Anticipation | Warm candlelight | Cling | The Doorman |
| The feast | Abundance, then too much | Red and gold | Swoon | The Glutton |
| The gardens | Sensual, overgrown | Green and cyan bioluminescence | Lure | The Sleeper |
| The ballroom | Euphoria, flow | Pink and gold, on the beat | Plunge | The Dancer |
| The baths | Dissolution | Teal through steam | Submerge, then Drift | The Bather |
| After hours | Comedown, loneliness | Cold, flickering white | Numb | The Last Toast |
| The centre | The question itself | Black, one red lamp | | The Host |

Dawn follows as a short epilogue, in daylight, with no enemies and no light sources at all.

The three guests who each embody an answer are bosses.
The Dancer wants the most intense pleasure and is burning out.
The Bather wants only the absence of pain and has stopped moving.
The Last Toast insists the party means something and gives speeches to an empty room.
The Doorman, the Glutton and the Sleeper are the loops the vision names:
the guest who keeps eating and the sleeper who lashes out when woken.

That makes seven bosses, not the eight in the content budget.
The budget counted a boss for each of seven areas plus the Host,
but the Host is the centre's boss, so one fewer is the honest count.

## The order of abilities

The player starts with the Lunge, the sabre and the Toast.
Movement is the pleasure, so the first minute has to feel good,
and the feel lab was tuned with the Lunge in hand.
Arriving late and in a hurry is impatience, which is the Lunge's theme.

| Order | Ability | Found | Opens first | Opens again, going back |
|---|---|---|---|---|
| Start | Lunge | | Wide gaps, curtains to dash through | |
| 1 | Cling | The Arrival's coat-room shaft | The Arrival's stair towers | Shafts everywhere |
| 2 | Swoon | The feast, after the Glutton | High galleries; the gardens' upper terraces | Ledges in the Arrival |
| 3 | Lure | The gardens, from a glowing flower | The ballroom; the gardens' canopy | Every lit lantern since the start |
| 4 | Plunge | The ballroom, as the Dancer's fight ends | Through the dance floor into the cellars | Cracked floors in the feast and the gardens |
| 5 | Submerge | The flooded cellars under the ballroom | The baths | Nectar pools in the feast and the gardens |
| 6 | Drift | The baths, riding steam | The service stairs up to after hours | Long crossings over the gardens |
| 7 | Numb | After hours, after the Last Toast | The glass and thorns round the centre | Thorn hedges in the gardens, glass in the Arrival |

The gardens teach the Lure gently, with glowing flowers close together and low.
The ballroom then asks for it at speed, which is where the vision puts the fastest movement in the game.
Plunge comes out of the Dancer's fight, which ends with the floor giving way;
the player falls into the next area instead of walking to it.

Every ability opens something new, and something old.
Going back to an earlier area with a new ability always finds a reward there:
a secret room, an upgrade, a shortcut, or a lounge.

## How open the order is

The order above is the intended night, not a fixed route.
After the Arrival, the feast and the gardens' lower half are both open,
so the player chooses which party to join first.
Each gate is an ability, never a key or a story flag,
except the centre, which also waits until its three guests have been met.

Some breaks are allowed on purpose and listed in the world file, so the checker knows them.
A skilled player can reach the ballroom's lower floor before the Lure with wall jumps and the Swoon,
and leave the gardens' Lure for later.
Breaks never skip a boss that gives an ability, so nobody can miss one.

## Weapons

| Weapon | Found | What else it does |
|---|---|---|
| The sabre | At the start | |
| The Jeroboam | The feast, on the Glutton's table | Shatters cracked statues and glass doors that block a way |
| The candelabrum | The ballroom, from its own chandeliers | Lights unlit sconces, which stay lit and become Lure points |

Weapons are not gates in the metroidvania sense, because each opens only optional ways.
The candelabrum is the exception that the theme asks for:
the Lure only goes where something glows, and the candelabrum lets the player make things glow.
Unlit sconces appear in every area, so carrying fire back opens secrets everywhere.

## Upgrades

Two kinds of upgrade hide in the rooms, as rewards for exploring and for going back.

Carnations raise health. There are six, pinned where someone left them.
Goblets raise how much Nectar the player can carry, so a High can start sooner and heal more.
There are six of those too.

There is no experience and no currency.
Nectar is the only resource, it is spent, never saved up for a shop,
and losing it on a blackout would be a punishment the vision rules out.

## The fade as a number

The fade is the share of colour taken from lit surfaces, from 0 to 1.
Each ability found adds its step, and the steps grow,
so the early game is barely touched and Numb takes the most.

| Ability | Step | Fade after it |
|---|---|---|
| Cling | 0.04 | 0.04 |
| Swoon | 0.08 | 0.12 |
| Lure | 0.12 | 0.24 |
| Plunge | 0.14 | 0.38 |
| Submerge | 0.16 | 0.54 |
| Drift | 0.18 | 0.72 |
| Numb | 0.28 | 1.00 |

The Lunge costs nothing, since the player arrives with it.
Dawn is the one place the fade lifts: daylight brings the colour back without a High,
which is the last thing the player sees and the last question the game asks.

## The two endings

The centre holds the Host. Meeting the Host offers the night again:
a new game that keeps the abilities and the fade, so the party goes on in grey.
The other way out is the Arrival's front door, locked since the start, which opens once Numb is found.
It leads to the dawn epilogue, a short walk in daylight with nothing left to do.
Neither ending is marked as the good one.

## Size and pace

About 150 rooms over five to ten hours, so roughly an hour to an hour and a half per area.

| Area | Rooms | Lounges |
|---|---|---|
| The Arrival | 14 | 2 |
| The feast | 24 | 3 |
| The gardens | 26 | 3 |
| The ballroom | 22 | 3 |
| The baths | 22 | 3 |
| After hours | 22 | 3 |
| The centre | 12 | 1 |
| Dawn | 4 | 0 |

## What it means for the vertical slice

The slice is the ballroom as a player would reach it:
arriving with the Lunge, Cling, Swoon and the Lure, at a fade of 0.24.
Inside it are the candelabrum, a Carnation, a Goblet, three lounges,
the Dancer, and Plunge as the reward, which ends the slice by dropping the player through the floor.

Building it needs four things the game does not have yet:
rooms joined by doors in a world file, abilities as pickups that the fade counts,
saving at lounges, and the reachability checker,
which can grow from the route search in `crates/world/tests/ballroom.rs`.

## Waiting for the director

- Starting with the Lunge, or with nothing and finding it in the Arrival.
- The order of the night above, in particular the gardens teaching the Lure before the ballroom.
- Seven bosses, with the three answer-guests among them.
- The candelabrum lighting sconces as Lure points.
- Carnations and Goblets as the only upgrades, with no currency.
- The fade's steps, and whether Dawn lifts it.
- A new game from the Host's ending that keeps the fade.
