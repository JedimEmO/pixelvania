//! The end of the slice, and how long the night has lasted.
//!
//! The first time the player reaches the cellar with the Plunge, the room holds a moment,
//! the curtain closes all the way, and the slice's last line shows on the black.
//! Then an end card, over the paused game: how long the night lasted, the rooms seen,
//! the Favours found and the blackouts. Jump keeps playing: the curtain opens where the
//! player stood, and the save remembers the end was seen, so it does not come again.
//! The Lunge, pressed twice, starts a new night: the save is deleted and the game starts over.

use bevy::prelude::*;
use world::Ability;

use crate::input::TickInput;
use crate::progress::PlayerProgress;
use crate::prompt::{Control, Device};
use crate::render::{Lighting, ScreenCamera};
use crate::rooms::ActiveRoom;

pub struct EndingPlugin;

impl Plugin for EndingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ending>()
            .add_systems(Update, (count_time, (begin, play, choose, show).chain()));
    }
}

/// The room where the slice ends.
const ROOM: &str = "cellar";
/// Seconds in the cellar before the curtain starts closing, so the fall has landed.
const HOLD: f32 = 2.5;
/// Seconds for the curtain to close, and to open again after the card.
const CLOSE: f32 = 2.5;
const OPEN: f32 = 1.5;
/// Seconds the last line shows on the black before the card.
const LINE: f32 = 4.5;

/// Where the ending is.
#[derive(Resource, Default, Debug)]
enum Ending {
    #[default]
    None,
    /// The room holds, the curtain closes, and the line shows, this many seconds in.
    Closing(f32),
    /// The card is up and the game paused. `confirming`: the Lunge was pressed once.
    Card { confirming: bool },
    /// Back to the night, the curtain opening over [`OPEN`].
    Opening,
}

#[derive(Component)]
struct Card;

#[derive(Component)]
struct CardText;

const PANEL: Color = Color::srgba(0.04, 0.02, 0.05, 0.0);
const GOLD: Color = Color::srgb(1.0, 0.82, 0.45);
const CREAM: Color = Color::srgb(0.95, 0.88, 0.8);

/// Adds the time played, which a paused game does not count.
fn count_time(time: Res<Time<Virtual>>, mut progress: ResMut<PlayerProgress>) {
    progress.0.seconds += time.delta_secs_f64();
}

/// Starts the ending the first time the player stands in the cellar with the Plunge.
fn begin(
    active: Res<ActiveRoom>,
    travel: Res<crate::travel::Travel>,
    mut progress: ResMut<PlayerProgress>,
    mut ending: ResMut<Ending>,
) {
    if active.name != ROOM
        || active.room.is_none()
        || progress.0.ended
        || !progress.0.has(Ability::Plunge)
        || !matches!(*travel, crate::travel::Travel::Idle)
        || !matches!(*ending, Ending::None)
    {
        return;
    }
    progress.0.ended = true;
    *ending = Ending::Closing(0.0);
    info!("the slice ends");
}

/// The curtain and the line, then the card.
fn play(
    mut commands: Commands,
    real: Res<Time<Real>>,
    mut ending: ResMut<Ending>,
    mut lighting: ResMut<Lighting>,
    mut time: ResMut<Time<Virtual>>,
    screen: Query<Entity, With<ScreenCamera>>,
    windows: Query<&Window>,
) {
    let dt = real.delta_secs();
    let Ok(camera) = screen.single() else { return };
    let scale = windows
        .single()
        .map_or(2.0, |w| (w.height() / 360.0).floor().max(1.0));
    match *ending {
        Ending::Closing(t) => {
            let t = t + dt;
            lighting.curtain = lighting.curtain.max(((t - HOLD) / CLOSE).clamp(0.0, 1.0));
            let line_at = HOLD + CLOSE;
            if t >= line_at && t - dt < line_at {
                commands.spawn(crate::story::scene_line(
                    &crate::story::words().ending,
                    LINE,
                    camera,
                    scale,
                ));
            }
            if t >= line_at + LINE {
                time.pause();
                commands.spawn(card(camera, scale));
                *ending = Ending::Card { confirming: false };
            } else {
                *ending = Ending::Closing(t);
            }
        }
        Ending::Card { .. } => lighting.curtain = 1.0,
        Ending::Opening => {
            lighting.curtain = (lighting.curtain - dt / OPEN).max(0.0);
            if lighting.curtain <= 0.0 {
                *ending = Ending::None;
            }
        }
        Ending::None => {}
    }
}

fn card(camera: Entity, scale: f32) -> impl Bundle {
    (
        Card,
        UiTargetCamera(camera),
        GlobalZIndex(25),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Node {
                padding: UiRect::all(px(14.0 * scale)),
                border: UiRect::all(px(scale)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(GOLD),
            children![(
                CardText,
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(9.0 * scale),
                    ..default()
                },
                TextColor(CREAM),
            )],
        )],
    )
}

/// Jump keeps playing; the Lunge twice starts a new night.
#[allow(clippy::too_many_arguments)]
fn choose(
    mut commands: Commands,
    mut ending: ResMut<Ending>,
    mut input: ResMut<TickInput>,
    mut time: ResMut<Time<Virtual>>,
    progress: Res<PlayerProgress>,
    indulgence: Res<crate::indulge::Indulgence>,
    last: Res<crate::lounge::LastLounge>,
    cards: Query<Entity, With<Card>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ending::Card { confirming } = *ending else {
        return;
    };
    // The game is paused, so presses wait here unconsumed; the card takes them.
    if std::mem::take(&mut input.jump_pressed) {
        for e in &cards {
            commands.entity(e).despawn();
        }
        time.unpause();
        if let Some(lounge) = &last.0 {
            crate::lounge::save(&progress.0, lounge, &indulgence);
        }
        *ending = Ending::Opening;
    } else if std::mem::take(&mut input.lunge_pressed) {
        if confirming {
            info!("a new night");
            crate::lounge::delete_save();
            start_over(&mut exit);
        } else {
            *ending = Ending::Card { confirming: true };
        }
    }
    // Nothing else pressed over the card should act once play resumes.
    input.attack_pressed = false;
    input.toast_pressed = false;
    input.lure_pressed = false;
    input.plunge_pressed = false;
    input.map_pressed = false;
}

/// Starts the game over from nothing: the page reloads on the web, and the game runs itself
/// again on the desktop.
fn start_over(exit: &mut MessageWriter<AppExit>) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = exit;
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::current_exe() {
            Ok(exe) => {
                if let Err(e) = std::process::Command::new(exe)
                    .args(std::env::args_os().skip(1))
                    .spawn()
                {
                    warn!("cannot start the game again: {e}");
                }
            }
            Err(e) => warn!("cannot find the game to start it again: {e}"),
        }
        exit.write(AppExit::Success);
    }
}

/// The card's words: the night in numbers, and the choice.
fn show(
    ending: Res<Ending>,
    progress: Res<PlayerProgress>,
    device: Res<Device>,
    world_rooms: Res<crate::map::WorldRooms>,
    assets: Res<Assets<crate::rooms::RoomAsset>>,
    mut texts: Query<&mut Text, With<CardText>>,
) {
    let Ending::Card { confirming } = *ending else {
        return;
    };
    let Ok(mut text) = texts.single_mut() else {
        return;
    };
    let p = &progress.0;
    let rooms = crate::rooms::world().rooms.len();
    let found = p.taken.iter().filter(|t| t.ends_with(":favour")).count() as u32
        + crate::boss::BOSS_FAVOURS * p.beaten.len() as u32;
    let jump = Control::Jump.key(*device);
    let lunge = Control::Lunge.key(*device);
    let choice = if confirming {
        format!("{lunge} again to forget this night and start a new one    {jump} keep playing")
    } else {
        format!("{jump} keep playing    {lunge} start a new night")
    };
    let s = format!(
        "THE NIGHT GOES ON\n\n\
         This is where the slice ends. Thank you for coming.  - H.\n\n\
         The night so far      {}\n\
         Rooms seen            {} of {rooms}\n\
         Favours found         {found} of {}\n\
         Level                 {}\n\
         Blackouts             {}\n\n\
         {choice}",
        clock(p.seconds),
        p.visited.len(),
        favours_in_slice(&world_rooms, &assets),
        p.stats.level(),
        p.blackouts,
    );
    if text.0 != s {
        text.0 = s;
    }
}

/// Every Favour the night holds so far: those in its rooms, and the bosses'.
fn favours_in_slice(
    rooms: &crate::map::WorldRooms,
    assets: &Assets<crate::rooms::RoomAsset>,
) -> u32 {
    rooms
        .0
        .iter()
        .filter_map(|(_, handle)| assets.get(handle))
        .map(|room| {
            let favours = room
                .0
                .entities_of("pickup")
                .filter(|d| d.props.get("item").map(String::as_str) == Some("favour"))
                .count() as u32;
            favours + crate::boss::BOSS_FAVOURS * room.0.entities_of("boss").count() as u32
        })
        .sum::<u32>()
}

/// Seconds as `1:02:03`, or `2:03` under an hour.
fn clock(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_reads_hours_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(125.9), "2:05");
        assert_eq!(clock(3723.0), "1:02:03");
    }
}
