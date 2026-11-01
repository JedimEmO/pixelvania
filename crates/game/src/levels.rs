//! The lounge's menu: where Favours are spent on levels.
//!
//! Resting at a lounge opens it, over the paused game, and fills the Cordials.
//! Up and down pick a stat, jump spends a Favour on a level of it,
//! and the Lunge or the map button leaves, which saves again.
//! Favours are found, not earned: hidden in the rooms, and given by the bosses.

use bevy::prelude::*;
use world::Stat;

use crate::indulge::Indulgence;
use crate::input::TickInput;
use crate::progress::PlayerProgress;
use crate::render::ScreenCamera;

pub struct LevelsPlugin;

impl Plugin for LevelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoungeMenu>()
            .add_systems(Update, (open, navigate, show).chain());
    }
}

/// Whether the menu is asked for, and which stat is picked.
#[derive(Resource, Default)]
pub struct LoungeMenu {
    /// Set by the lounge when the player rests.
    pub wanted: bool,
    selected: usize,
    /// The stick was pushed past the threshold last frame.
    stick: bool,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct MenuText;

const PANEL: Color = Color::srgba(0.06, 0.03, 0.07, 0.92);
const GOLD: Color = Color::srgb(1.0, 0.82, 0.45);
const CREAM: Color = Color::srgb(0.95, 0.88, 0.8);

fn open(
    mut commands: Commands,
    mut menu: ResMut<LoungeMenu>,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<(), With<MenuRoot>>,
    screen: Query<Entity, With<ScreenCamera>>,
    windows: Query<&Window>,
) {
    if !menu.wanted || !roots.is_empty() {
        return;
    }
    let Ok(camera) = screen.single() else { return };
    let scale = windows
        .single()
        .map_or(2.0, |w| (w.height() / 360.0).floor().max(1.0));
    menu.selected = 0;
    // The up press that rested is still held: it should not move the selection.
    menu.stick = true;
    time.pause();
    commands.spawn((
        MenuRoot,
        UiTargetCamera(camera),
        GlobalZIndex(20),
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
                MenuText,
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(9.0 * scale),
                    ..default()
                },
                TextColor(CREAM),
            )],
        )],
    ));
}

#[allow(clippy::too_many_arguments)]
fn navigate(
    mut commands: Commands,
    mut menu: ResMut<LoungeMenu>,
    mut input: ResMut<TickInput>,
    mut progress: ResMut<PlayerProgress>,
    indulgence: Res<Indulgence>,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<MenuRoot>>,
    last: Res<crate::lounge::LastLounge>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let Ok(root) = roots.single() else { return };
    let y = input.move_axis.y;
    let pushed = y.abs() > 0.6;
    if pushed && !menu.stick {
        let n = Stat::ALL.len();
        menu.selected = if y > 0.0 {
            (menu.selected + n - 1) % n
        } else {
            (menu.selected + 1) % n
        };
        sfx.write(crate::audio::PlaySfx {
            pitch: 2.0,
            volume: 0.4,
            ..crate::audio::PlaySfx::new(crate::audio::Sfx::Footstep)
        });
    }
    menu.stick = pushed;
    // The game is paused, so presses wait here unconsumed; the menu takes them.
    if std::mem::take(&mut input.jump_pressed) {
        let stat = Stat::ALL[menu.selected];
        if progress.0.level_up(stat) {
            sfx.write(crate::audio::PlaySfx {
                pitch: 1.2,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureCatch)
            });
            info!("{} raised to {}", stat.name(), progress.0.stats.get(stat));
        } else {
            sfx.write(crate::audio::PlaySfx {
                pitch: 0.6,
                volume: 0.6,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::HitLight)
            });
        }
    }
    let leave = std::mem::take(&mut input.lunge_pressed) || std::mem::take(&mut input.map_pressed);
    if leave {
        commands.entity(root).despawn();
        menu.wanted = false;
        time.unpause();
        if let Some(lounge) = &last.0 {
            crate::lounge::save(&progress.0, lounge, &indulgence);
        }
    }
    // Nothing else the player pressed while resting should act once play resumes.
    input.attack_pressed = false;
    input.toast_pressed = false;
    input.lure_pressed = false;
    input.plunge_pressed = false;
}

fn show(
    menu: Res<LoungeMenu>,
    progress: Res<PlayerProgress>,
    device: Res<crate::prompt::Device>,
    mut texts: Query<&mut Text, With<MenuText>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };
    let stats = &progress.0.stats;
    let favours = progress.0.favours;
    let plural = if favours == 1 { "" } else { "s" };
    let mut s = format!(
        "RESTING\n\nCordials refilled    Level {}\n{favours} Favour{plural} to spend\n\n",
        stats.level(),
    );
    for (i, stat) in Stat::ALL.into_iter().enumerate() {
        let mark = if i == menu.selected { ">" } else { " " };
        s.push_str(&format!(
            "{mark} {:<10} {:>2}   {}\n",
            stat.name(),
            stats.get(stat),
            stat.effect()
        ));
    }
    use crate::prompt::Control;
    let leave = format!("{} leave", Control::Lunge.key(*device));
    if favours > 0 {
        s.push_str(&format!(
            "\n{} spend a Favour    {leave}",
            Control::Jump.key(*device)
        ));
    } else {
        s.push_str(&format!("\nFavours are found in the rooms    {leave}"));
    }
    if text.0 != s {
        text.0 = s;
    }
}
