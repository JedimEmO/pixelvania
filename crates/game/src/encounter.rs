//! A boss fight as the rest of the game hears and reads it.
//!
//! When the player comes within sight of the Dancer, the fight's music takes over from the room's
//! and builds with each phase, and she speaks a line as each phase begins and one as she falls,
//! from `assets/story.ron`. When she is beaten the music gives way to silence and then the room.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use world::boss::Phase;

use crate::boss::{BossDefeated, Dancer};
use crate::combat::Fighting;
use crate::music::Encounter;
use crate::player::Player;

pub struct EncounterPlugin;

impl Plugin for EncounterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (engage, speak, fade_speech));
    }
}

/// How close the player must come for the fight to begin, in pixels.
const ENGAGE: f32 = 240.0;

fn phase_number(p: Phase) -> u8 {
    match p {
        Phase::Waltz => 1,
        Phase::Frenzy => 2,
        Phase::Burnout => 3,
    }
}

/// Starts the fight's music when the player comes near, and follows its phase.
fn engage(
    mut encounter: ResMut<Encounter>,
    dancers: Query<(&Dancer, &Fighting, &Transform)>,
    players: Query<&Transform, With<Player>>,
    mut engaged: Local<bool>,
) {
    let Some((d, f, t)) = dancers.iter().next() else {
        if *engaged {
            *engaged = false;
            encounter.track = None;
        }
        return;
    };
    if f.combat.health <= 0.0 {
        if encounter.track.is_some() {
            encounter.track = None;
        }
        return;
    }
    let near = players
        .single()
        .is_ok_and(|p| p.translation.truncate().distance(t.translation.truncate()) < ENGAGE);
    if near {
        *engaged = true;
    }
    if *engaged {
        let phase = phase_number(d.phase());
        if encounter.track.as_deref() != Some("dancer") || encounter.phase != phase {
            encounter.track = Some("dancer".into());
            encounter.phase = phase;
        }
    }
}

/// A line spoken above the speaker's head, fading out.
#[derive(Component)]
struct Speech {
    age: f32,
}

fn speech(text: &str, at: Vec2) -> impl Bundle {
    (
        Speech { age: 0.0 },
        Text2d::new(text.to_owned()),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.6, 0.8)),
        RenderLayers::layer(crate::render::OVERLAY_LAYER),
        Transform::from_translation((at + Vec2::Y * 78.0).round().extend(6.0)),
    )
}

/// Seconds a spoken line shows, with its fade.
const SPEECH_TIME: f32 = 3.0;

/// The Dancer speaks as each phase begins and as she falls.
fn speak(
    mut commands: Commands,
    mut defeated: MessageReader<BossDefeated>,
    dancers: Query<(Entity, &Dancer, &Transform)>,
    players: Query<&Transform, With<Player>>,
    mut said: Local<Option<(Entity, Phase)>>,
) {
    let lines = &crate::story::words().dancer;
    for d in defeated.read() {
        commands.spawn(speech(&lines.fall, d.at));
    }
    for (e, d, t) in &dancers {
        let near = players
            .single()
            .is_ok_and(|p| p.translation.truncate().distance(t.translation.truncate()) < ENGAGE);
        if !near || *said == Some((e, d.phase())) {
            continue;
        }
        *said = Some((e, d.phase()));
        let line = match d.phase() {
            Phase::Waltz => &lines.waltz,
            Phase::Frenzy => &lines.frenzy,
            Phase::Burnout => &lines.burning_out,
        };
        commands.spawn(speech(line, t.translation.truncate()));
    }
}

fn fade_speech(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut lines: Query<(Entity, &mut Speech, &mut TextColor)>,
) {
    for (e, mut s, mut color) in &mut lines {
        s.age += time.delta_secs();
        if s.age >= SPEECH_TIME {
            commands.entity(e).despawn();
            continue;
        }
        let k = ((SPEECH_TIME - s.age) / 0.6).min(1.0);
        color.0 = Color::srgba(1.0, 0.6, 0.8, k);
    }
}
