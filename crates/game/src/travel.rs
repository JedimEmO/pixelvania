//! Passing through doors from one room to the next.
//!
//! Walking into a door draws a black curtain over the screen. Behind it, the next room loads
//! and is built, the player is put just inside the door it leads to, facing into the room,
//! and the camera jumps to them. Then the curtain opens again.
//! The player keeps their running speed through a side door, so a run carries on.
//!
//! While a boss is standing, its room's doors stay shut: the fight is finished, one way or the other.
//!
//! A blackout uses the same passage, with a slower curtain:
//! the defeated player is taken to the last lounge and gets up there.

use bevy::prelude::*;

use crate::camera::Follow;
use crate::combat::Fighting;
use crate::player::Player;
use crate::render::Lighting;
use crate::rooms::ActiveRoom;

pub struct TravelPlugin;

impl Plugin for TravelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Travel>()
            .add_systems(Update, travel.before(crate::player::place));
    }
}

/// Seconds for the curtain to close and to open, through a door and in a blackout.
const CLOSE: f32 = 0.12;
const OPEN: f32 = 0.2;
const BLACKOUT_CLOSE: f32 = 0.9;
const BLACKOUT_OPEN: f32 = 1.2;

/// Where a passage arrives.
#[derive(Debug, Clone)]
pub enum Arrival {
    /// Just inside a door, through it.
    Door(String),
    /// At a lounge, getting up from a blackout.
    Wake(String),
}

/// Where a passage through a door is.
#[derive(Resource, Default, Debug)]
pub enum Travel {
    #[default]
    Idle,
    /// The curtain is closing before going to this room.
    Closing { room: String, to: Arrival },
    /// The next room is loading behind the closed curtain.
    Loading { to: Arrival },
    /// The player has arrived and the curtain is opening, over this many seconds.
    Opening { seconds: f32 },
}

impl Travel {
    /// Blacks out and wakes at a lounge.
    pub fn blackout(lounge: &crate::lounge::LoungeRef) -> Travel {
        Travel::Closing {
            room: lounge.room.clone(),
            to: Arrival::Wake(lounge.id.clone()),
        }
    }
}

fn travel(
    time: Res<Time<Real>>,
    assets: Res<AssetServer>,
    mut state: ResMut<Travel>,
    mut active: ResMut<ActiveRoom>,
    mut lighting: ResMut<Lighting>,
    mut follow: ResMut<Follow>,
    mut players: Query<(&mut Player, &mut Fighting)>,
    bosses: Query<&Fighting, (With<crate::boss::Dancer>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    match &*state {
        Travel::Idle => {
            let (Some(room), Ok((p, _))) = (&active.room, players.single()) else {
                return;
            };
            if bosses.iter().any(|f| f.combat.health > 0.0) {
                return;
            }
            let b = &p.ctrl.body;
            let doors = room.doors().unwrap_or_default();
            let through = doors
                .iter()
                .find(|d| d.touches(b.x as f32, b.y as f32, b.w as f32, b.h as f32));
            if let Some(d) = through {
                *state = Travel::Closing {
                    room: d.to_room.clone(),
                    to: Arrival::Door(d.to_door.clone()),
                };
            }
        }
        Travel::Closing { room, to } => {
            let seconds = match to {
                Arrival::Door(_) => CLOSE,
                Arrival::Wake(_) => BLACKOUT_CLOSE,
            };
            lighting.curtain = (lighting.curtain + dt / seconds).min(1.0);
            if lighting.curtain >= 1.0 {
                let to = to.clone();
                active.go_to(&assets, &room.clone());
                *state = Travel::Loading { to };
            }
        }
        Travel::Loading { to } => {
            let Some(room) = &active.room else { return };
            let (arrival, seconds) = match to {
                Arrival::Door(door) => (room.door(door).and_then(|d| d.arrival(room)), OPEN),
                Arrival::Wake(id) => (
                    crate::lounge::lounge_feet(room, id).map(|f| (f, 1)),
                    BLACKOUT_OPEN,
                ),
            };
            if let (Some((feet, facing)), Ok((mut p, mut f))) = (arrival, players.single_mut()) {
                p.arrive(feet, facing);
                if matches!(to, Arrival::Wake(_)) {
                    p.ctrl.vx = 0.0;
                    crate::player::revive(&mut f);
                }
            } else {
                warn!("room `{}` has nowhere to arrive for {to:?}", room.name);
            }
            follow.snap();
            *state = Travel::Opening { seconds };
        }
        Travel::Opening { seconds } => {
            lighting.curtain = (lighting.curtain - dt / *seconds).max(0.0);
            if lighting.curtain <= 0.0 {
                *state = Travel::Idle;
            }
        }
    }
}
