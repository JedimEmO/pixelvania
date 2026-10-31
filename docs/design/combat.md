# Combat

Combat is heavy.
Every swing is a commitment, every hit lands with mass behind it,
and enemies react like bodies, not health bars.
Movement is still the pleasure of exploring;
combat is the pleasure of impact.

This page covers what makes combat feel heavy, the weapons, enemies and bosses,
and how the High and the Comedown change the weight of a hit.
The movement it builds on is in [game feel](game-feel.md).

## What weight is made of

Weight is not one feature.
It comes from several systems agreeing with each other,
and if one of them disagrees, the whole hit feels light.

Commitment.
An attack cannot be cancelled before its hit frame.
After a hit connects, you may cancel the recovery into a Lunge, and only into a Lunge.
A missed swing plays its full recovery.
Swinging is a decision with a cost, which is what makes it feel like a heavy object is moving.

Anticipation.
Every attack has a wind-up in the opposite direction to the blow,
and a big change of silhouette on the release frame.
pixelfx's acts describe exactly this, and its timing data drives ours.

Hit-stop.
On contact, the attacker freezes while the victim shivers in place.
Heavier blows freeze longer.

| Blow | Hit-stop |
|---|---|
| Light | 0.04 s |
| Heavy | 0.09 s |
| Charged, parry, finisher | 0.15 to 0.2 s |

Direction.
The screen shakes along the line of the hit, not randomly.
Light blows rumble the small, fast motor; heavy blows the large, slow one.
Heavy hits are mixed with a sub-bass thump and a short drop of the music's volume,
so the impact takes room in the mix.

Reaction.
Enemies have mass and poise.
Light hits chip poise and make them flinch.
Breaking poise staggers them, heavy blows knock them down or launch them,
and a launched enemy that hits a wall splats against it, takes damage and bounces off.
A body thrown into another enemy knocks that one too.

The world reacts as well.
Furniture breaks, glasses shatter, chandeliers fall when their chain is cut,
and every break spills debris that bounces on the real floor through pixelfx's simulation.

## The player's moves

| Move | Input | Notes |
|---|---|---|
| Combo | Attack, up to three times | The third hit is slower and heavier |
| Charged attack | Hold attack, release | Armoured while swinging: hits do not interrupt it |
| Rising attack | Up + attack | Launches light enemies |
| Air combo | Attack in the air | Two hits, holds you briefly in the air |
| Pogo | Down + attack in the air | Bounces off enemies and hazards |
| Plunge attack | Plunge while above an enemy | Ground slam with a shockwave; breaks poise |
| Toast | Parry button, timed against a blow | A 0.12 s window; staggers the attacker outright |
| Finisher | Attack a staggered enemy | A short, framed kill that bursts Nectar |

The Toast is a dedicated button, separate from attacking.
A parry that shares the attack button punishes players for attacking,
and heavy combat needs both decisions to stay clean.

## Weapons

There are three weapons, and the player switches between them at any time.
Each one changes how a fight is played, not only how much damage it does.

The sabre is the first weapon and the balanced one.
Quick, with a long reach for its speed, and the best parry follow-up.
Its finisher is a sabrage stroke that ends with the pop of a cork.

The Jeroboam is a champagne bottle as long as you are tall, swung like a maul.
Very slow, huge poise damage, and the heaviest hit-stop in the game.
Its charged attack is a ground slam;
a full charge shakes the bottle, and the finisher sprays foam that blinds nearby enemies.

The candelabrum is a polearm with burning candles at its head.
Long sweeping arcs that hit everything in front of you, and fire that keeps burning on the floor.
It controls space rather than single targets.

## The mood in combat

The High makes every hit heavier:
more hit-stop, more knockback, more poise damage, a deeper thump.
It is the best combat in the game, and it is supposed to feel that way.

The Comedown makes hits hollow.
Hit-stop shortens, knockback drops, the thump loses its low end, and enemies shrug off blows they would have staggered from.
Nothing about your timing changes; only the weight is taken away.
This is where the theme speaks loudest, because you feel the comedown in your hands.

Finishers are the main source of Nectar.
Violence pays in pleasure, and the pleasure pays for the next High.
The game never comments on that loop; it only makes it available.

## Enemies

Enemies are guests lost in their own loops.
The game needs about twenty-five types across the seven areas,
each built on the shared rig with its own proportions, parts and weapon.

Every enemy attack is readable from its silhouette within its first three frames,
and has a sound cue on its wind-up.
Nothing in combat depends on colour alone,
because colour drains from the world as the game goes on.

Enemies come in weight classes.
Light guests are launched and splatted.
Medium guests stagger and fall.
Heavy guests only stagger when their poise breaks, and are never launched;
they are what the Jeroboam is for.

## Bosses

One boss closes each area, and the Host waits at the centre: eight in all.
Each boss fight takes place in an arena built for it,
with things to break, chandeliers to Lure from, and at least one way to use the environment against the boss.

Bosses have several phases, and each phase change is staged:
a stagger, a moment of music, a change in the arena.
Their poise is a visible part of the fight;
breaking it is the main goal, and a broken boss takes a finisher that is different for each one.

## Tuning and tools

Each weapon and enemy attack is a table of frame data in a RON file:
wind-up, active frames, recovery, cancel window, hit-stop, poise damage, knockback.
It reloads while the game runs, like the movement tuning.

Development builds draw hitboxes, hurtboxes and poise bars on demand,
and can pause the game and step it one tick at a time.
A combat lab room spawns any enemy against any weapon,
and fights can be recorded and replayed against new tuning.

## Difficulty

Fights should be demanding, and the game offers ways to soften them without a separate easy mode.
An assist menu can reduce damage taken, slow the game down, and widen the Toast window.
Assists never lock content or achievements.
