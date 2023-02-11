use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    math::Vec3A,
    prelude::*,
};
use iyes_loopless::prelude::ConditionSet;

use crate::{level::level_instance::LevelInstance, GameState};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Game)
                .run_if_resource_exists::<LevelInstance>()
                .with_system(camera_zoom_scroll_system)
                .with_system(camera_pan_system)
                .into(),
        );
    }
}

pub fn camera_zoom_scroll_system(
    mut scroll_event: EventReader<MouseWheel>,
    mut camera: Query<&mut GlobalTransform, With<Camera>>,
) {
    let Ok(mut camera_transform) = camera.get_single_mut() else {
        return;
    };

    let forward: Vec3A = camera_transform.forward().into();

    for event in scroll_event.iter() {
        match event.unit {
            MouseScrollUnit::Line => {
                *camera_transform.translation_mut() += 0.5 * event.y * forward;
            }
            MouseScrollUnit::Pixel => {
                *camera_transform.translation_mut() += 0.05 * event.y * forward;
            }
        }
    }
}

pub fn camera_pan_system(
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
        let offset = 0.05 * Vec3A::new(event.delta.x, 0.0, event.delta.y);
        *camera_transform.translation_mut() -= offset;
    }
}
