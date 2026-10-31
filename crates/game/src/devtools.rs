//! Tools for checking and tuning the game.
//!
//! `PIXELVANIA_SCREENSHOT=path.png` saves a screenshot after the game has settled, then exits.
//! `PIXELVANIA_HOLD=right,jump` holds those inputs from the start, for screenshots of motion;
//! `attack`, `charge` and `toast` press those at set frames, and `jeroboam` or `candelabrum` switch to that weapon.
//! `PIXELVANIA_RECORD=path` writes every tick's input to a file;
//! `PIXELVANIA_REPLAY=path` plays such a file back instead of reading the controller.
//! `PIXELVANIA_FX=sword-slash` spawns that effect in front of the player once it has landed;
//! the number keys 1 to 6 spawn each pixelfx effect there at any time.
//! F2 pauses the game and F3, while paused, steps it one gameplay tick.
//! F4 cycles the mood of combat: neutral, the High and the Comedown.
//! The keys 7, 8 and 9 spawn a light, medium or heavy guest in front of the player.
//! F5 shows hurtboxes, active hitboxes, and health and poise bars; `PIXELVANIA_BOXES=1` starts with them on.
//! `PIXELVANIA_MOOD=high` or `comedown` starts in that mood.
//! A replay is exact as long as the room and the tuning are the same as when it was recorded.
//!
//! With the `dev` feature, F1 opens a panel that edits the tuning live and saves it to the file.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Write};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use world::Controls;

use crate::input::{TickControls, TickInput, TickSet};
use crate::player::Player;

pub struct DevToolsPlugin;

/// Frames to wait before the screenshot: long enough for assets to load and the camera to settle.
const SCREENSHOT_FRAME: u32 = 150;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(path) = std::env::var("PIXELVANIA_SCREENSHOT") {
            let frame_at = std::env::var("PIXELVANIA_SCREENSHOT_FRAME")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(SCREENSHOT_FRAME);
            app.insert_resource(ScreenshotPlan { path, frame_at })
                .add_systems(Update, screenshot_and_exit);
        }
        if let Ok(hold) = std::env::var("PIXELVANIA_HOLD") {
            let held: Vec<String> = hold.split(',').map(|s| s.trim().to_owned()).collect();
            app.insert_resource(HeldInputs(held))
                .add_systems(PreUpdate, hold_inputs.after(crate::input::collect));
        }
        if let Ok(path) = std::env::var("PIXELVANIA_REPLAY") {
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    let ticks: VecDeque<Controls> =
                        text.lines().filter_map(Controls::decode).collect();
                    info!("replaying {} ticks from {path}", ticks.len());
                    app.insert_resource(Replay(ticks)).add_systems(
                        FixedUpdate,
                        replay.in_set(TickSet::Input).after(record_or_replay_point),
                    );
                }
                Err(e) => error!("cannot read replay {path}: {e}"),
            }
        }
        if let Ok(path) = std::env::var("PIXELVANIA_RECORD") {
            match File::create(&path) {
                Ok(file) => {
                    info!("recording input to {path}");
                    app.insert_resource(Recorder(BufWriter::new(file)))
                        .add_systems(
                            FixedUpdate,
                            record.in_set(TickSet::Input).after(record_or_replay_point),
                        );
                }
                Err(e) => error!("cannot record to {path}: {e}"),
            }
        }
        let planned = std::env::var("PIXELVANIA_FX")
            .ok()
            .and_then(|v| pixelfx::EffectId::parse(v.trim()));
        app.add_systems(
            Update,
            (pause_and_step, cycle_mood, draw_boxes, spawn_guests),
        )
        .insert_resource(ShowBoxes(std::env::var("PIXELVANIA_BOXES").is_ok()))
        .add_systems(Startup, boxes_in_glow);
        match std::env::var("PIXELVANIA_MOOD").as_deref() {
            Ok("high") => app.insert_resource(crate::combat::Mood::ALL[1]),
            Ok("comedown") => app.insert_resource(crate::combat::Mood::ALL[2]),
            _ => app,
        };
        app.insert_resource(PlannedFx(planned))
            .add_systems(Update, spawn_test_fx.before(crate::player::react));
        app.add_systems(
            FixedUpdate,
            record_or_replay_point
                .in_set(TickSet::Input)
                .after(crate::input::take_controls),
        );
        #[cfg(feature = "dev")]
        panel::build(app);
    }
}

/// Marks the point in the tick after the live input is taken, where replay and recording run.
fn record_or_replay_point() {}

#[derive(Resource)]
struct ScreenshotPlan {
    path: String,
    frame_at: u32,
}

#[derive(Resource)]
struct HeldInputs(Vec<String>);

#[derive(Resource)]
struct Replay(VecDeque<Controls>);

#[derive(Resource)]
struct Recorder(BufWriter<File>);

fn screenshot_and_exit(
    mut commands: Commands,
    plan: Res<ScreenshotPlan>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == plan.frame_at {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(plan.path.clone()));
    }
    // Saving happens a few frames later, on another thread.
    if *frame == plan.frame_at + 30 {
        exit.write(AppExit::Success);
    }
}

fn hold_inputs(held: Res<HeldInputs>, mut input: ResMut<TickInput>, mut frame: Local<u32>) {
    *frame += 1;
    for h in &held.0 {
        match h.as_str() {
            "left" => input.move_axis.x = -1.0,
            "right" => input.move_axis.x = 1.0,
            "down" => input.move_axis.y = -1.0,
            "jump" => {
                input.jump_held = true;
                // Press once the player has had time to spawn and land.
                input.jump_pressed |= *frame == 60;
            }
            "lunge" => input.lunge_pressed |= *frame == 90,
            // Three presses, timed to chain the combo.
            "attack" => input.attack_pressed |= [70, 84, 100].contains(&*frame),
            // Held for a second, then released into the charged attack.
            "charge" => {
                input.attack_pressed |= *frame == 70;
                input.attack_held = (70..130).contains(&*frame);
            }
            "toast" => input.toast_pressed |= *frame == 80,
            "jeroboam" => input.switch_weapon_pressed = *frame == 40,
            "candelabrum" => input.switch_weapon_pressed = *frame == 40 || *frame == 50,
            _ => {}
        }
    }
}

#[derive(Resource)]
struct ShowBoxes(bool);

/// 7, 8 and 9 spawn a light, medium and heavy guest four tiles in front of the player.
fn spawn_guests(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&Player>,
    weapons: Option<Res<crate::combat::Weapons>>,
    active: Res<crate::rooms::ActiveRoom>,
    mut images: ResMut<Assets<Image>>,
) {
    use anim::guest::Class;
    let (Ok(p), Some(weapons), Some(room)) = (players.single(), weapons, &active.room) else {
        return;
    };
    for (key, class) in [
        (KeyCode::Digit7, Class::Light),
        (KeyCode::Digit8, Class::Medium),
        (KeyCode::Digit9, Class::Heavy),
    ] {
        if keys.just_pressed(key) {
            let b = &p.ctrl.body;
            let x = b.x as f32 + b.w as f32 / 2.0 + p.ctrl.facing as f32 * 64.0;
            let feet = (x, (b.y + b.h) as f32);
            // A guest spawned inside a wall could never get out.
            let (w, h) = class.size();
            let body = world::Body::new(x.round() as i32 - w / 2, feet.1 as i32 - h, w, h);
            if body.collides_at(room, 0, 0) {
                info!("no room for a {} guest there", class.name());
                continue;
            }
            crate::enemies::spawn_guest(&mut commands, class, feet, &weapons, &mut images);
        }
    }
}

/// Overlays draw into the glow buffer, so the scene's lighting does not dim them.
fn boxes_in_glow(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.render_layers = bevy::camera::visibility::RenderLayers::layer(crate::render::GLOW_LAYER);
    config.line.width = 1.0;
}

/// Draws each fighter's hurtbox, its active hitboxes, and bars for health and poise.
fn draw_boxes(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowBoxes>,
    mut gizmos: Gizmos,
    fighters: Query<(
        &crate::combat::Fighting,
        Option<&Player>,
        Option<&crate::enemies::Guest>,
    )>,
) {
    use crate::combat::HasBody;
    if keys.just_pressed(KeyCode::F5) {
        show.0 = !show.0;
    }
    if !show.0 {
        return;
    }
    let rect = |gizmos: &mut Gizmos, r: world::combat::Rect, color: Color| {
        let centre = Vec2::new(r.x + r.w / 2.0, -(r.y + r.h / 2.0));
        gizmos.rect_2d(
            Isometry2d::from_translation(centre),
            Vec2::new(r.w, r.h),
            color,
        );
    };
    for (f, p, g) in &fighters {
        let Some(body) = p.map(HasBody::body).or(g.map(HasBody::body)) else {
            continue;
        };
        let hurt = world::combat::Rect::from_body(body);
        let shown = if f.combat.helpless() {
            Color::srgb(0.9, 0.8, 0.2)
        } else {
            Color::srgb(0.2, 0.8, 0.4)
        };
        rect(&mut gizmos, hurt, shown);
        let feet = (
            body.x as f32 + body.w as f32 / 2.0,
            (body.y + body.h) as f32,
        );
        for hit in f.combat.attacker.active_hitboxes(feet) {
            rect(&mut gizmos, hit, Color::srgb(1.0, 0.2, 0.2));
        }
        // Health above poise, each 24 px wide, over the head.
        let top = -(body.y as f32) + 6.0;
        let left = feet.0 - 12.0;
        let bar = |gizmos: &mut Gizmos, y: f32, k: f32, color: Color| {
            gizmos.line_2d(
                Vec2::new(left, y),
                Vec2::new(left + 24.0, y),
                Color::srgb(0.2, 0.2, 0.25),
            );
            if k > 0.0 {
                gizmos.line_2d(
                    Vec2::new(left, y),
                    Vec2::new(left + 24.0 * k.min(1.0), y),
                    color,
                );
            }
        };
        let c = &f.combat;
        bar(
            &mut gizmos,
            top + 3.0,
            c.health / f.max_health,
            Color::srgb(0.9, 0.3, 0.35),
        );
        bar(
            &mut gizmos,
            top,
            c.poise / c.max_poise,
            Color::srgb(0.95, 0.85, 0.4),
        );
    }
}

/// F4 moves combat to the next mood.
fn cycle_mood(keys: Res<ButtonInput<KeyCode>>, mut mood: ResMut<crate::combat::Mood>) {
    if keys.just_pressed(KeyCode::F4) {
        *mood = mood.next();
        info!("combat mood: {}", mood.name);
    }
}

/// F2 pauses and resumes; F3 runs one gameplay tick while paused.
fn pause_and_step(world: &mut World) {
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let (toggle, step) = (
        keys.just_pressed(KeyCode::F2),
        keys.just_pressed(KeyCode::F3),
    );
    let mut time = world.resource_mut::<Time<Virtual>>();
    if toggle {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
        info!(
            "game {}",
            if time.is_paused() {
                "paused"
            } else {
                "running"
            }
        );
    }
    if step && time.is_paused() {
        world.run_schedule(bevy::app::FixedMain);
    }
}

#[derive(Resource)]
struct PlannedFx(Option<pixelfx::EffectId>);

/// Frame at which `PIXELVANIA_FX` fires: after the player has spawned and settled.
const PLANNED_FX_FRAME: u32 = 80;

/// Spawns effects in front of the player, for looking at them in the room.
fn spawn_test_fx(
    keys: Res<ButtonInput<KeyCode>>,
    planned: Res<PlannedFx>,
    players: Query<(&Player, &Transform)>,
    mut fx: MessageWriter<crate::effects::SpawnFx>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    let Ok((p, t)) = players.single() else { return };
    const KEYS: [KeyCode; 6] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ];
    let mut ids: Vec<pixelfx::EffectId> = KEYS
        .iter()
        .zip(pixelfx::EffectId::ALL)
        .filter(|(k, _)| keys.just_pressed(**k))
        .map(|(_, id)| id)
        .collect();
    if *frame == PLANNED_FX_FRAME {
        ids.extend(planned.0);
    }
    let facing = p.ctrl.facing;
    for id in ids {
        let pos = t.translation.truncate() + Vec2::new(facing as f32 * 14.0, 0.0);
        fx.write(crate::effects::SpawnFx::new(id, pos).facing(facing));
    }
}

/// Replays recorded input once the player exists, so the recording starts where it started.
fn replay(
    mut replay: ResMut<Replay>,
    mut controls: ResMut<TickControls>,
    players: Query<(), With<Player>>,
    mut done: Local<bool>,
) {
    if players.is_empty() || *done {
        return;
    }
    match replay.0.pop_front() {
        Some(c) => controls.0 = c,
        None => {
            *done = true;
            info!("replay finished");
        }
    }
}

fn record(
    mut recorder: ResMut<Recorder>,
    controls: Res<TickControls>,
    players: Query<(), With<Player>>,
    mut ticks: Local<u32>,
) {
    if players.is_empty() {
        return;
    }
    let line = controls.0.encode();
    if let Err(e) = writeln!(recorder.0, "{line}") {
        error!("recording failed: {e}");
    }
    *ticks += 1;
    if (*ticks).is_multiple_of(120) {
        let _ = recorder.0.flush();
    }
}

#[cfg(feature = "dev")]
mod panel {
    //! The tuning panel. F1 shows and hides it; `PIXELVANIA_PANEL=1` starts with it open.

    use bevy::prelude::*;
    use bevy_egui::{
        EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext,
        egui,
    };

    use crate::render::ScreenCamera;
    use crate::tuning::Tuning;

    pub fn build(app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .insert_resource(Open(std::env::var("PIXELVANIA_PANEL").is_ok()))
            .add_systems(Update, (attach, toggle))
            .add_systems(EguiPrimaryContextPass, draw);
    }

    #[derive(Resource)]
    struct Open(bool);

    /// The panel draws on the screen camera, not on one of the low-resolution world cameras.
    fn attach(mut commands: Commands, cameras: Query<Entity, Added<ScreenCamera>>) {
        for camera in &cameras {
            commands.entity(camera).insert(PrimaryEguiContext);
        }
    }

    fn toggle(keys: Res<ButtonInput<KeyCode>>, mut open: ResMut<Open>) {
        if keys.just_pressed(KeyCode::F1) {
            open.0 = !open.0;
        }
    }

    macro_rules! sliders {
        ($ui:expr, $s:expr, { $($field:ident : $lo:expr => $hi:expr),* $(,)? }) => {
            $( $ui.add(egui::Slider::new(&mut $s.$field, $lo..=$hi).text(stringify!($field))); )*
        };
    }

    fn pair(ui: &mut egui::Ui, label: &str, v: &mut (f32, f32)) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut v.0).speed(0.01).range(0.2..=2.0));
            ui.add(egui::DragValue::new(&mut v.1).speed(0.01).range(0.2..=2.0));
            ui.label(label);
        });
    }

    fn draw(
        mut contexts: EguiContexts,
        open: Res<Open>,
        mut tuning: ResMut<Tuning>,
        mut mood: ResMut<crate::combat::Mood>,
    ) -> Result {
        if !open.0 {
            return Ok(());
        }
        let mut edited = tuning.clone();
        egui::Window::new("Tuning")
            .default_width(360.0)
            .vscroll(true)
            .show(contexts.ctx_mut()?, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Save to tuning.ron").clicked() {
                        match super::save_tuning(&edited) {
                            Ok(path) => info!("saved tuning to {path}"),
                            Err(e) => error!("saving tuning failed: {e}"),
                        }
                    }
                    if ui.button("Defaults").clicked() {
                        edited = Tuning::default();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Combat mood (F4)");
                    for m in crate::combat::Mood::ALL {
                        if ui.selectable_label(mood.name == m.name, m.name).clicked() {
                            *mood = m;
                        }
                    }
                });
                let m = &mut edited.movement;
                ui.collapsing("Running", |ui| {
                    sliders!(ui, m, {
                        run_speed: 50.0 => 300.0,
                        run_accel: 200.0 => 6000.0,
                        run_decel: 200.0 => 6000.0,
                        turn_accel: 200.0 => 8000.0,
                        walk_threshold: 0.05 => 0.9,
                        down_threshold: 0.2 => 0.95,
                    });
                });
                ui.collapsing("Jumping and falling", |ui| {
                    sliders!(ui, m, {
                        jump_speed: 100.0 => 600.0,
                        jump_cut: 0.0 => 1.0,
                        gravity: 200.0 => 3000.0,
                        max_fall: 100.0 => 800.0,
                        apex_threshold: 0.0 => 150.0,
                        apex_gravity: 0.1 => 1.0,
                        fast_fall_gravity: 1.0 => 3.0,
                        fast_fall_max: 100.0 => 1000.0,
                        air_accel: 100.0 => 6000.0,
                        air_decel: 0.0 => 6000.0,
                        air_overspeed_decel: 0.0 => 3000.0,
                    });
                });
                ui.collapsing("Forgiveness", |ui| {
                    sliders!(ui, m, {
                        coyote_time: 0.0 => 0.3,
                        jump_buffer: 0.0 => 0.3,
                        corner_correction: 0 => 8,
                        ledge_pop: 0 => 8,
                    });
                });
                ui.collapsing("Lunge", |ui| {
                    sliders!(ui, m, {
                        lunge_speed: 100.0 => 900.0,
                        lunge_time: 0.02 => 0.5,
                        lunge_cooldown: 0.0 => 1.0,
                    });
                });
                ui.collapsing("Swoon", |ui| {
                    sliders!(ui, m, {
                        swoon_speed: 50.0 => 600.0,
                        swoon_apex_gravity: 0.05 => 1.0,
                        swoon_apex_threshold: 0.0 => 200.0,
                    });
                });
                ui.collapsing("Cling", |ui| {
                    sliders!(ui, m, {
                        wall_slide_max: 0.0 => 400.0,
                        wall_jump_x: 0.0 => 500.0,
                        wall_jump_y: 0.0 => 600.0,
                        wall_jump_lock: 0.0 => 0.5,
                        wall_coyote: 0.0 => 0.3,
                    });
                });
                let c = &mut edited.camera;
                ui.collapsing("Camera", |ui| {
                    sliders!(ui, c, {
                        follow_rate_x: 0.5 => 30.0,
                        follow_rate_y: 0.5 => 30.0,
                        lead: 0.0 => 160.0,
                        lead_speed: 10.0 => 600.0,
                        fall_release: 0.0 => 200.0,
                        rise_release: 0.0 => 300.0,
                        look_up: -100.0 => 100.0,
                    });
                });
                ui.collapsing("Juice", |ui| {
                    let j = &mut edited.juice;
                    sliders!(ui, j, {
                        squash_recovery: 1.0 => 60.0,
                        afterimage_life: 0.0 => 1.0,
                        rumble: 0.0 => 2.0,
                        shake: 0.0 => 2.0,
                    });
                    pair(ui, "jump stretch", &mut j.jump_stretch);
                    pair(ui, "land squash", &mut j.land_squash);
                    pair(ui, "lunge stretch", &mut j.lunge_stretch);
                    sliders!(ui, edited.movement, { hard_landing_speed: 50.0 => 800.0 });
                });
            });
        if edited != *tuning {
            *tuning = edited;
        }
        Ok(())
    }
}

/// Writes the tuning back to `assets/tuning.ron`. Returns the path written.
#[cfg(feature = "dev")]
fn save_tuning(tuning: &crate::tuning::Tuning) -> Result<String, Box<dyn std::error::Error>> {
    let root = std::env::var("BEVY_ASSET_ROOT").unwrap_or_else(|_| ".".into());
    let path = std::path::Path::new(&root).join("assets/tuning.ron");
    let body = ron::ser::to_string_pretty(tuning, ron::ser::PrettyConfig::default())?;
    let text = format!(
        "// Tuning values, saved from the tuning panel. Saved changes apply while the game runs,\n\
         // with the `dev` feature. The meaning of each value is documented in the code:\n\
         // `world::MoveTuning`, and `CameraTuning` and `Juice` in crates/game/src/tuning.rs.\n\
         {body}\n"
    );
    std::fs::write(&path, text)?;
    Ok(path.display().to_string())
}
