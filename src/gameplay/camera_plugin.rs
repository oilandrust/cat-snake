use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    math::Vec3Swizzles,
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
        let new_pos = (camera_transform.translation()
            - 0.1 * Vec3::new(event.delta.x, 0.0, event.delta.y))
        .xyz();

        let new_pos = new_pos.extend(camera_transform.translation().z);
        *camera_transform.translation_mut() = new_pos.into();
    }
}
