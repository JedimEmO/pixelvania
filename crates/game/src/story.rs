//! The story: guests who talk, the Host's cards, and the slice's few staged moments.
//!
//! Guests who talk are room entities, `npc x y id=attendant`, drawn on the guest rigs and never hostile.
//! The Host's cards are `note x y id=cloakroom`, small glowing cards that float above the floor.
//! A prompt shows over either when the player is close, `[W] Talk` or `[UP] Read`; pressing up
//! opens a box at the bottom of the screen and pauses the game. Jump or up shows the next line;
//! the last one closes it, and the box says which button does it.
//! The words are in `assets/story.ron`, decided in `docs/design/story.md`.
//!
//! The staged moments: a new game opens on black with one line, and fades up slowly;
//! and each room of the world shows its name the first time it is entered.
//! The slice's ending, in the cellar, is in `ending.rs`.

use std::collections::BTreeMap;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use serde::Deserialize;
use world::Ability;

use crate::input::{TickControls, TickInput, TickSet};
use crate::player::{Look, Player};
use crate::progress::PlayerProgress;
use crate::prompt::{Control, Device, Prompt};
use crate::render::{GLOW_LAYER, Light2d, ScreenCamera};
use crate::rooms::{ActiveRoom, RoomPart};

pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        let waking = crate::lounge::startup_save().is_none()
            && ![
                "PIXELVANIA_ROOM",
                "PIXELVANIA_SPAWN",
                "PIXELVANIA_PROGRESS",
                "PIXELVANIA_REPLAY",
            ]
            .iter()
            .any(|v| std::env::var(v).is_ok());
        app.init_resource::<Dialogue>()
            .init_resource::<NearTalk>()
            .insert_resource(Scene {
                waking: waking.then_some(0.0),
            })
            .add_systems(
                FixedUpdate,
                talk.in_set(TickSet::Simulate)
                    .in_set(Talk)
                    .after(crate::combat::SimulateBodies),
            )
            .add_systems(
                Update,
                (spawn, face_and_mark, open_box, advance, scenes, titles),
            );
    }
}

/// Talking happens before resting, so a card on a lounge is read rather than rested on.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Talk;

/// The words, from `assets/story.ron`.
#[derive(Deserialize)]
pub struct Words {
    guests: BTreeMap<String, Guest>,
    cards: BTreeMap<String, String>,
    waking: String,
    pub ending: String,
    pub dancer: DancerLines,
}

#[derive(Deserialize)]
struct Guest {
    name: String,
    look: String,
    lines: Vec<Lines>,
}

#[derive(Deserialize)]
struct Lines {
    when: Vec<Ability>,
    say: Vec<String>,
}

/// What the Dancer says, one line per phase and one as she falls.
#[derive(Deserialize, Clone)]
pub struct DancerLines {
    pub waltz: String,
    pub frenzy: String,
    pub burning_out: String,
    pub fall: String,
}

/// The words, read once.
pub fn words() -> &'static Words {
    static WORDS: std::sync::OnceLock<Words> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        ron::from_str(include_str!("../../../assets/story.ron")).expect("assets/story.ron parses")
    })
}

impl Guest {
    /// The lines for what has been found: the last set whose abilities are all found.
    fn say(&self, progress: &world::Progress) -> Vec<String> {
        self.lines
            .iter()
            .rev()
            .find(|l| l.when.iter().all(|a| progress.has(*a)))
            .map(|l| l.say.clone())
            .unwrap_or_default()
    }
}

/// Something the player can read or talk to.
#[derive(Component)]
struct Talkable {
    /// A guest's id, or a card's.
    id: String,
    card: bool,
    /// Where it stands, in pixels with y down.
    feet: (f32, f32),
}

/// Whether the player is close enough to talk to someone or read something,
/// so the lounge does not also offer a rest that up would not give.
#[derive(Resource, Default, PartialEq)]
pub struct NearTalk(pub bool);

/// The prompt over something the player is close enough to talk to or read.
#[derive(Component)]
struct Mark;

/// How close counts as close, in pixels.
const REACH: f32 = 24.0;

fn rig_for(look: &str) -> anim::Rig {
    use anim::guest::Class;
    let class = match look {
        "light" => Class::Light,
        "heavy" => Class::Heavy,
        "spinner" => Class::Spinner,
        "toaster" => Class::Toaster,
        _ => Class::Medium,
    };
    anim::guest::rig(class)
}

fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    mut spawned: Local<Option<u64>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(room) = &active.room else { return };
    if *spawned == Some(active.build) {
        return;
    }
    *spawned = Some(active.build);
    let w = words();
    for def in room.entities_of("npc") {
        let id = def.props.get("id").cloned().unwrap_or_default();
        let Some(guest) = w.guests.get(&id) else {
            error!(
                "room `{}`, line {}: no guest `{id}` in assets/story.ron",
                room.name, def.line
            );
            continue;
        };
        let feet = def.pixel_pos();
        let look = Look::new(rig_for(&guest.look), &mut images);
        let image = look.image.clone();
        commands.spawn((
            RoomPart,
            Name::new(format!("guest {id}")),
            Talkable {
                id: id.clone(),
                card: false,
                feet,
            },
            look,
            Transform::from_xyz(feet.0.round(), -feet.1.round(), 0.9),
            Visibility::default(),
            children![
                (
                    Sprite::from_image(image),
                    crate::player::look_anchor(),
                    Transform::default(),
                ),
                (
                    Mark,
                    Prompt::bundle(Control::Up, "Talk", Vec2::new(0.0, 50.0)),
                ),
            ],
        ));
    }
    for def in room.entities_of("note") {
        let id = def.props.get("id").cloned().unwrap_or_default();
        if !w.cards.contains_key(&id) {
            error!(
                "room `{}`, line {}: no card `{id}` in assets/story.ron",
                room.name, def.line
            );
            continue;
        }
        let at = def.pixel_pos();
        commands.spawn((
            RoomPart,
            Name::new(format!("card {id}")),
            Talkable {
                id,
                card: true,
                // The card floats; the reader stands on the floor below it.
                feet: (at.0, at.1 + 16.0),
            },
            Transform::from_xyz(at.0.round(), -at.1.round(), 2.0),
            Visibility::default(),
            Light2d {
                color: LinearRgba::rgb(1.0, 0.85, 0.6),
                radius: 40.0,
                strength: 0.9,
                glow: 0.5,
            },
            children![
                (
                    Sprite::from_color(Color::srgb(1.0, 0.92, 0.75), Vec2::new(7.0, 5.0)),
                    RenderLayers::layer(GLOW_LAYER),
                    Transform::default(),
                ),
                (
                    Mark,
                    Prompt::bundle(Control::Up, "Read", Vec2::new(0.0, 12.0)),
                ),
            ],
        ));
    }
}

fn feet_of(p: &Player) -> (f32, f32) {
    let b = &p.ctrl.body;
    ((b.x + b.w / 2) as f32, (b.y + b.h) as f32)
}

fn near(p: &Player, t: &Talkable) -> bool {
    let (x, y) = feet_of(p);
    (x - t.feet.0).abs() < REACH && (y - t.feet.1).abs() < 20.0 && p.ctrl.grounded
}

/// Guests idle and turn to the player; marks show over what is close enough.
#[allow(clippy::too_many_arguments)]
fn face_and_mark(
    time: Res<Time>,
    players: Query<&Player>,
    mut talkers: Query<(&Talkable, Option<&mut Look>, &Children)>,
    mut marks: Query<&mut Visibility, With<Mark>>,
    dialogue: Res<Dialogue>,
    mut near_talk: ResMut<NearTalk>,
    mut images: ResMut<Assets<Image>>,
    mut frame_no: Local<u32>,
) {
    *frame_no = frame_no.wrapping_add(1);
    let player = players.single().ok();
    let mut any = false;
    for (t, look, children) in &mut talkers {
        let close = player.is_some_and(|p| near(p, t)) && !dialogue.is_open();
        any |= close;
        for &c in children {
            if let Ok(mut vis) = marks.get_mut(c) {
                vis.set_if_neq(if close {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
        if let Some(mut look) = look {
            let facing = player.map_or(1, |p| if feet_of(p).0 < t.feet.0 { -1 } else { 1 });
            let motion = anim::Motion {
                grounded: true,
                facing,
                run_speed: 100.0,
                ..anim::Motion::default()
            };
            look.animator.update(motion, time.delta_secs_f64());
            let pose = look.animator.pose();
            look.draw(&pose, false, false, &mut images, *frame_no);
        }
    }
    near_talk.set_if_neq(NearTalk(any));
}

/// The open box, if any: who speaks, their lines, and how far through them it is.
#[derive(Resource, Default)]
pub struct Dialogue {
    speaker: String,
    lines: Vec<String>,
    line: usize,
    /// Characters of the current line shown so far.
    shown: f32,
    /// Asked for this tick; the box opens on the next frame.
    wanted: bool,
}

impl Dialogue {
    pub fn is_open(&self) -> bool {
        !self.lines.is_empty()
    }
}

/// Presses up beside something to talk to it, in the gameplay tick, before resting.
fn talk(
    input: Res<TickControls>,
    progress: Res<PlayerProgress>,
    players: Query<&Player>,
    talkers: Query<&Talkable>,
    mut dialogue: ResMut<Dialogue>,
    mut was_up: Local<bool>,
) {
    let up = input.0.y > 0.6;
    let pressed = up && !*was_up;
    *was_up = up;
    if !pressed || dialogue.is_open() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Some(t) = talkers.iter().find(|t| near(p, t)) else {
        return;
    };
    let w = words();
    let (speaker, lines) = if t.card {
        ("A card".to_owned(), vec![w.cards[&t.id].clone()])
    } else {
        let g = &w.guests[&t.id];
        (g.name.clone(), g.say(&progress.0))
    };
    if lines.is_empty() {
        return;
    }
    *dialogue = Dialogue {
        speaker,
        lines,
        line: 0,
        shown: 0.0,
        wanted: true,
    };
}

/// Whether the player is talking, so the lounge does not also rest on the same press.
pub fn talking(dialogue: &Dialogue) -> bool {
    dialogue.is_open()
}

#[derive(Component)]
struct DialogueBox;

#[derive(Component)]
struct DialogueName;

#[derive(Component)]
struct DialogueText;

/// The line under the text saying which button goes on.
#[derive(Component)]
struct DialogueHint;

const PANEL: Color = Color::srgba(0.05, 0.03, 0.06, 0.92);
const GOLD: Color = Color::srgb(1.0, 0.82, 0.45);
const CREAM: Color = Color::srgb(0.95, 0.88, 0.8);

fn open_box(
    mut commands: Commands,
    mut dialogue: ResMut<Dialogue>,
    mut time: ResMut<Time<Virtual>>,
    screen: Query<Entity, With<ScreenCamera>>,
    windows: Query<&Window>,
) {
    if !dialogue.wanted {
        return;
    }
    dialogue.wanted = false;
    let Ok(camera) = screen.single() else { return };
    let scale = windows
        .single()
        .map_or(2.0, |w| (w.height() / 360.0).floor().max(1.0));
    time.pause();
    let font = |size: f32| TextFont {
        font_size: bevy::text::FontSize::Px(size * scale),
        ..default()
    };
    commands.spawn((
        DialogueBox,
        UiTargetCamera(camera),
        GlobalZIndex(20),
        Node {
            position_type: PositionType::Absolute,
            left: percent(15),
            right: percent(15),
            bottom: px(24.0 * scale),
            padding: UiRect::axes(px(12.0 * scale), px(8.0 * scale)),
            border: UiRect::all(px(scale)),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.0 * scale),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(GOLD),
        children![
            (DialogueName, Text::new(""), font(8.0), TextColor(GOLD)),
            (DialogueText, Text::new(""), font(10.0), TextColor(CREAM)),
            (
                DialogueHint,
                Text::new(""),
                font(7.0),
                TextColor(GOLD.with_alpha(0.7)),
                Node {
                    align_self: AlignSelf::FlexEnd,
                    ..default()
                },
            ),
        ],
    ));
}

/// Shows the current line a few letters at a time; jump or up finishes it, then moves on.
#[allow(clippy::too_many_arguments)]
fn advance(
    mut commands: Commands,
    real: Res<Time<Real>>,
    mut input: ResMut<TickInput>,
    mut dialogue: ResMut<Dialogue>,
    mut time: ResMut<Time<Virtual>>,
    boxes: Query<Entity, With<DialogueBox>>,
    mut names: Query<&mut Text, (With<DialogueName>, Without<DialogueText>)>,
    mut texts: Query<&mut Text, (With<DialogueText>, Without<DialogueName>)>,
    mut hints: Query<
        &mut Text,
        (
            With<DialogueHint>,
            Without<DialogueName>,
            Without<DialogueText>,
        ),
    >,
    device: Res<Device>,
    mut was_up: Local<bool>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let Ok(root) = boxes.single() else {
        *was_up = input.move_axis.y > 0.6;
        return;
    };
    let line = dialogue.lines[dialogue.line].clone();
    let len = line.chars().count() as f32;
    let before = dialogue.shown as usize;
    dialogue.shown = (dialogue.shown + real.delta_secs() * 45.0).min(len);
    if dialogue.shown as usize / 4 != before / 4 && (dialogue.shown as usize) < len as usize {
        sfx.write(crate::audio::PlaySfx {
            pitch: 2.6,
            volume: 0.12,
            ..crate::audio::PlaySfx::new(crate::audio::Sfx::Footstep)
        });
    }
    let up = input.move_axis.y > 0.6;
    let pressed = std::mem::take(&mut input.jump_pressed)
        || std::mem::take(&mut input.attack_pressed)
        || (up && !*was_up);
    *was_up = up;
    if pressed {
        if dialogue.shown < len {
            dialogue.shown = len;
        } else if dialogue.line + 1 < dialogue.lines.len() {
            dialogue.line += 1;
            dialogue.shown = 0.0;
        } else {
            commands.entity(root).despawn();
            *dialogue = Dialogue::default();
            time.unpause();
            return;
        }
    }
    if let Ok(mut name) = names.single_mut()
        && name.0 != dialogue.speaker
    {
        name.0 = dialogue.speaker.clone();
    }
    if let Ok(mut hint) = hints.single_mut() {
        let last = dialogue.line + 1 == dialogue.lines.len();
        let want = format!(
            "{} {}",
            Control::Jump.key(*device),
            if last { "close" } else { "next" }
        );
        if hint.0 != want {
            hint.0 = want;
        }
    }
    if let Ok(mut text) = texts.single_mut() {
        let shown: String = line.chars().take(dialogue.shown as usize).collect();
        if text.0 != shown {
            text.0 = shown;
        }
    }
}

/// The staged moments in progress.
#[derive(Resource)]
struct Scene {
    /// Seconds into waking, for a new game.
    waking: Option<f32>,
}

/// A line of text centred on the screen, for scenes.
#[derive(Component)]
struct SceneLine {
    age: f32,
    life: f32,
}

/// Drawn on the screen over everything, so it shows on the closed curtain.
pub(crate) fn scene_line(text: &str, life: f32, camera: Entity, scale: f32) -> impl Bundle {
    (
        SceneLine { age: 0.0, life },
        UiTargetCamera(camera),
        GlobalZIndex(30),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            top: percent(45),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new(text.to_owned()),
            TextFont {
                font_size: bevy::text::FontSize::Px(14.0 * scale),
                ..default()
            },
            TextColor(Color::srgba(0.95, 0.88, 0.8, 0.0)),
        )],
    )
}

/// Waking on a new game, and the scene lines fading in and out.
#[allow(clippy::too_many_arguments)]
fn scenes(
    mut commands: Commands,
    real: Res<Time<Real>>,
    mut scene: ResMut<Scene>,
    mut lighting: ResMut<crate::render::Lighting>,
    mut lines: Query<(Entity, &mut SceneLine, &Children)>,
    mut colors: Query<&mut TextColor>,
    screen: Query<Entity, With<ScreenCamera>>,
    windows: Query<&Window>,
) {
    let dt = real.delta_secs();
    let Ok(camera) = screen.single() else { return };
    let scale = windows
        .single()
        .map_or(2.0, |w| (w.height() / 360.0).floor().max(1.0));
    // Waking: black, a line, then the room fades up slowly.
    if let Some(t) = scene.waking {
        if t == 0.0 {
            commands.spawn(scene_line(&words().waking, 3.5, camera, scale));
        }
        let t = t + dt;
        lighting.curtain = (1.0 - ((t - 2.0) / 3.0)).clamp(0.0, 1.0);
        scene.waking = (t < 5.0).then_some(t);
    }
    for (e, mut line, children) in &mut lines {
        line.age += dt;
        if line.age >= line.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = (line.age / 0.8).min(1.0) * ((line.life - line.age) / 0.8).min(1.0);
        for &c in children {
            if let Ok(mut color) = colors.get_mut(c) {
                color.0 = Color::srgba(0.95, 0.88, 0.8, k);
            }
        }
    }
}

/// A room's name for its title: `card_room` becomes `The Card Room`.
pub fn title(name: &str) -> String {
    let words: Vec<String> = name
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect();
    format!("The {}", words.join(" "))
}

/// Shows a room's name the first time it is ever entered, once the curtain is open.
fn titles(
    mut commands: Commands,
    travel: Res<crate::travel::Travel>,
    mut first: ResMut<crate::map::FirstVisits>,
    scene: Res<Scene>,
) {
    if !matches!(*travel, crate::travel::Travel::Idle) || scene.waking.is_some() {
        return;
    }
    for name in first.0.drain(..) {
        commands.spawn(crate::progress::Banner::bundle_text(
            &title(&name).to_uppercase(),
            "",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_parse_and_lines_follow_what_is_found() {
        let w = words();
        let mut p = world::Progress::new_game();
        let attendant = &w.guests["attendant"];
        assert!(attendant.say(&p)[0].starts_with("Ticket"));
        p.abilities.insert(Ability::Lunge);
        assert!(attendant.say(&p)[0].starts_with("In a hurry"));
        p.abilities.insert(Ability::Lure);
        assert!(attendant.say(&p)[0].starts_with("You have the look"));
        assert_eq!(title("card_room"), "The Card Room");
    }
}
