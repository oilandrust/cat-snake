use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    math::Vec3Swizzles,
    prelude::*,
};
use iyes_loopless::prelude::{ConditionHelpers, ConditionSet, IntoConditionalSystem};

use crate::{
    level::{level_instance::LevelInstance, level_template::LevelTemplate},
    GameState,
};

use super::level_pluggin::{LevelEntity, StartLevelEventWithLevel};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            CoreStage::PreUpdate,
            camera_setup_system
                .run_in_state(GameState::Game)
                .run_if_resource_exists::<LevelInstance>(),
        )
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Game)
                .run_if_resource_exists::<LevelInstance>()
                .with_system(camera_zoom_scroll_system)
                .with_system(camera_pan_system)
                .into(),
        );
    }
}

fn camera_setup_system(
    mut commands: Commands,
    mut event_start_level: EventReader<StartLevelEventWithLevel>,
    level_template: Res<LevelTemplate>,
) {
    if event_start_level.iter().next().is_none() {
        return;
    }

    let level_center = Vec3::new(
        level_template.grid.width() as f32 * 0.5,
        0.0,
        level_template.grid.height() as f32 * 0.5,
    );

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(level_center + 10.0 * Vec3::Y + 5.0 * Vec3::Z)
                .looking_at(level_center, Vec3::Y),
            ..default()
        },
        LevelEntity,
    ));
}

fn camera_zoom_scroll_system(
    mut scroll_event: EventReader<MouseWheel>,
    mut camera: Query<&mut Transform, With<Camera>>,
) {
    let Ok(mut camera_transform) = camera.get_single_mut() else {
        return;
    };

    let forward = camera_transform.forward();

    for event in scroll_event.iter() {
        match event.unit {
            MouseScrollUnit::Line => {
                camera_transform.translation += 0.5 * event.y * forward;
            }
            MouseScrollUnit::Pixel => {
                camera_transform.translation += 0.05 * event.y * forward;
            }
        }
    }
}

fn camera_pan_system(
    mut motion_event: EventReader<MouseMotion>,
    buttons: Res<Input<MouseButton>>,
    mut camera: Query<&mut GlobalTransform, With<Camera>>,
) {
    if !buttons.pressed(MouseButton::Right) {
        return;
    }

    let Ok(mut camera_transform) = camera.get_single_mut() else {
        return;
    };

    for event in motion_event.iter() {
        let new_pos = (camera_transform.translation()
            - 0.1 * Vec3::new(event.delta.x, 0.0, event.delta.y))
        .xyz();

        let new_pos = new_pos.extend(camera_transform.translation().z);
        *camera_transform.translation_mut() = new_pos.into();
    }
}
