//! Passing through doors from one room to the next.
//!
//! Walking into a door draws a black curtain over the screen. Behind it, the next room loads
//! and is built, the player is put just inside the door it leads to, facing into the room,
//! and the camera jumps to them. Then the curtain opens again.
//! The player keeps their running speed through a side door, so a run carries on.

use bevy::prelude::*;

use crate::camera::Follow;
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

/// Seconds for the curtain to close and to open.
const CLOSE: f32 = 0.12;
const OPEN: f32 = 0.2;

/// Where a passage through a door is.
#[derive(Resource, Default, Debug)]
pub enum Travel {
    #[default]
    Idle,
    /// The curtain is closing before going to this room and door.
    Closing { room: String, door: String },
    /// The next room is loading behind the closed curtain.
    Loading { door: String },
    /// The player has arrived and the curtain is opening.
    Opening,
}

fn travel(
    time: Res<Time<Real>>,
    assets: Res<AssetServer>,
    mut state: ResMut<Travel>,
    mut active: ResMut<ActiveRoom>,
    mut lighting: ResMut<Lighting>,
    mut follow: ResMut<Follow>,
    mut players: Query<&mut Player>,
) {
    let dt = time.delta_secs();
    match &*state {
        Travel::Idle => {
            let (Some(room), Ok(p)) = (&active.room, players.single()) else {
                return;
            };
            let b = &p.ctrl.body;
            let doors = room.doors().unwrap_or_default();
            let through = doors
                .iter()
                .find(|d| d.touches(b.x as f32, b.y as f32, b.w as f32, b.h as f32));
            if let Some(d) = through {
                *state = Travel::Closing {
                    room: d.to_room.clone(),
                    door: d.to_door.clone(),
                };
            }
        }
        Travel::Closing { room, door } => {
            lighting.curtain = (lighting.curtain + dt / CLOSE).min(1.0);
            if lighting.curtain >= 1.0 {
                let door = door.clone();
                active.go_to(&assets, &room.clone());
                *state = Travel::Loading { door };
            }
        }
        Travel::Loading { door } => {
            let Some(room) = &active.room else { return };
            let arrival = room.door(door).and_then(|d| d.arrival(room));
            if let (Some((feet, facing)), Ok(mut p)) = (arrival, players.single_mut()) {
                p.arrive(feet, facing);
            } else {
                warn!("room `{}` has no door `{door}` to arrive at", room.name);
            }
            follow.snap();
            *state = Travel::Opening;
        }
        Travel::Opening => {
            lighting.curtain = (lighting.curtain - dt / OPEN).max(0.0);
            if lighting.curtain <= 0.0 {
                *state = Travel::Idle;
            }
        }
    }
}
