# Audio and music

How the game sounds, and how the music follows the player's mood.
The music has the same job as the light in [art direction](art-direction.md):
it is where the pleasure lives, and the comedown is heard before it is seen.

## Direction

The music is the party.
Lounge, exotica, Italo disco and early house, played lush and slightly wrong:
an FM electric piano, detuned analogue pads, a drum machine heard through a wall,
vibraphone, and a vocal pad that sounds like a choir a long way off.
The instruments are chosen for what synthesis does well,
because the music is composed and rendered as code (see [production](production.md)).

The party is diegetic wherever it can be.
Music comes from rooms, and you hear it through walls and floors before you find it.
Following the sound is a way to navigate,
and arriving at the source is a small reward of its own; the music becomes clear.

One short motif recurs across the game, a four-note toast.
It is played warm at the arrival, as a disco hook in the ballroom,
slowed and pitched down after hours, and as a single piano line at dawn.

## Each stage of the night

| Stage | Sound |
|---|---|
| Arrival | Lounge jazz in the next room, candle crackle, distant laughter |
| The feast | Harpsichord over a disco rhythm section, cutlery, too many voices |
| The gardens | Exotica: vibraphone, bird calls, slow percussion, humidity |
| The ballroom | Italo disco and house at full volume, the fastest tempo in the game |
| The baths | Dub: long echoes, sub bass, everything underwater |
| After hours | Earlier themes slowed down and pitched down, a fridge hum, one flickering light |
| Dawn | No music, then wind and birds, then the motif once |

## Mood mixing

Each piece of music is written as stems: drums, bass, harmony, lead, texture.
The mood decides which stems play and how they are processed.

At rest, a subset of stems plays.
A High brings in every stem, opens the filters and adds a little reverb shimmer.
A Comedown drops the drums, closes a low-pass filter over the mix,
and slows the playback by a few percent, so the music sags in pitch the way a turntable does when it loses power.
The progression fade slowly thins the default stem set over the whole game.

Every piece in an area shares a tempo and key with its neighbours.
Moving between rooms crossfades on the beat instead of cutting.

## Sound effects

Sound effects that carry pleasure are tuned to the current area's key:
nectar pickups, the Lure attaching, the Toast parry ringing.
A pickup during a disco track is a note in the song.
This is a small thing that makes the world feel like it agrees with you.

Impacts and footsteps are not tuned; they stay physical and dry.
During a Comedown, everything except the player's own sounds is muffled.

## Engine

We use `kira` directly, under a thin layer of our own, rather than a Bevy audio plugin.
The mood mixing needs mixer tracks with filters and reverb,
playback rate changes, and a clock to start stems and transitions on the beat.
`kira` has all of these, and it runs on desktop and on the web.

The Bevy wrappers either expose only part of `kira`
or, in the case of `bevy_seedling`, do not support Bevy 0.20 yet.
Owning the layer also means a Bevy upgrade never waits on an audio crate.

On the web, audio can only start after the player presses something.
The title screen asks for a button press before anything plays.

## Composing

Music is written as code by the agents and rendered offline into stems;
[production](production.md) describes the pipeline.
Agents cannot hear what they write, so every track goes to you for a listen before it ships,
and a track that does not work goes back with notes in words: too busy, too bright, the bass is mushy.

The stem structure and the tempo and key plan are fixed before any area's music is written,
because both shape how every track is composed.

## Ballroom sketch

`cargo xtask music` renders a 64-second stereo loop at 120 BPM in D minor, in 4/4.
The 32-bar form is A / A′ / B / A″, eight bars each;
the middle eight leave more space around the four-note toast, D–F–E–A.
Each chord lasts two bars: Dm9, B♭maj7, Fmaj7, C6,
Gm9, Dm9/A, B♭maj7, A7♭9. The progression repeats twice.

Five stems share one normalization gain, with their sum peaking at −6 dBFS:
soft four-on-the-floor drums and brushes, octave bass, detuned pads and FM keys,
a bell-like FM lead, and room noise with distant vowel-like murmurs.
Release tails and reverb wrap into the beginning of the loop.

On native builds, `music = ballroom` selects the theme in a room's `[room]` section.
Mood changes tween stem levels, low-pass cutoff and reverb over 1.5 seconds.
The High opens the filter and brightens the room; the Comedown drops drums and lead.
This sketch keeps a fixed playback rate, including during the Comedown.
Silent stems keep their place when leaving the room, and returning fades them back in.
Music is skipped on wasm. The sketch still needs a listening pass.

## The feast

`cargo xtask music` also renders the feast's room music: 64 seconds at the ballroom's 120 BPM,
so the rooms' lights keep the beat, in G minor.
A baroque circle of fifths, a bar a chord, turns under a four-on-the-floor:
Gm, Cm, F, B♭, E♭, Am7♭5, D7, Gm.
The five stems take the same mood roles as the ballroom's:
drums with claps and open hats, octave disco bass, a harpsichord continuo in sixteenths,
a running harpsichord figure as the lead, and cutlery and voices as the room.
The service stair hears it through the wall. It still needs a listening pass.

## Boss themes

A boss names its theme, and the encounter plays it in place of the room's music,
bringing in more of its stems with each phase of the fight.
Each stem takes one of the five mood roles, so the High and the Comedown mix a boss as they mix a room.

The Dancer's theme is 40 bars of 132 BPM disco in D minor, in eight stems:
drums, bass and pads first; the arp and the stabs from her second phase;
the lead, a choir and a double-time surge for the last.

The Doorman's is a slow processional, 32 bars at 104 BPM in C minor, on i, VI, iv and V.
The list is timpani with the three knocks of his staff, a dotted bass and a door chime of bells.
Impatience adds a harmonium in the same dotted rhythm, brass stabs on the offbeats and a marching snare.
Closing time adds a mocking fanfare, an octave up in the third section,
over a ticking clock and a hand bell calling last orders.
The Glutton's is a galop, 32 bars at 144 BPM in A minor.
Appetite is an oom-pah tuba, galloping drums and a harpsichord's broken chords.
Seconds adds cutlery that is never quite in time, a brass tune with a can-can's leer,
and a babble of voices talking over each other.
Too much adds an organ that will not let go and a low churning drone, with a sour cluster every other bar.
None of the three themes has had a listening pass.
