use std::{fs::File, io::Write};

use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_prototype_debug_lines::DebugShapes;
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem},
    state::NextState,
};
use ron::ser::PrettyConfig;

use crate::{
    despawn_with,
    gameplay::{
        level_pluggin::{
            clear_level_runtime_resources_system, spawn_level_entities_system, spawn_wall,
            CurrentLevelResourcePath, Food, GridEntity, LevelEntity, LevelLoadedEvent, Spike, Wall,
        },
        snake_pluggin::{
            spawn_snake_system, update_snake_transforms_system, MaterialMeshBuilder, Snake,
            SpawnSnakeEvent,
        },
    },
    level::{
        level_instance::LevelInstance,
        level_template::{LevelTemplate, LoadingLevel},
    },
    tools::{
        cameras::{camera_3d_free, EditorCamera},
        picking::PickingCameraBundle,
    },
    utils::ray_from_screen_space,
    GameAssets, GameState,
};

use super::{
    cameras::camera_3d_free::FlycamPlugin,
    picking::{DefaultPickingPlugins, Selection},
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(FlycamPlugin)
            .add_system(start_editor_system.run_in_state(GameState::Game))
            .add_system(stop_editor_system.run_in_state(GameState::Editor))
            .add_enter_system(GameState::Editor, init_level_instance_system)
            .add_exit_system(GameState::Editor, despawn_with::<LevelEntity>)
            .add_exit_system(GameState::Editor, clear_level_runtime_resources_system)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>()
                    .with_system(spawn_level_entities_system)
                    .with_system(spawn_snake_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .with_system(add_wall_on_click_system)
                    .with_system(delete_selected_wall_system)
                    .with_system(save_level_system)
                    .with_system(update_snake_transforms_system)
                    .with_system(move_selected_grid_entity)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>()
                    //.with_system(camera_zoom_scroll_system)
                    //.with_system(camera_pan_system)
                    .into(),
            );
    }
}

fn start_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
    mut spawn_snake_event: EventWriter<SpawnSnakeEvent>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(NextState(GameState::Editor));
    level_loaded_event.send(LevelLoadedEvent);
    spawn_snake_event.send(SpawnSnakeEvent);
}

fn init_level_instance_system(mut commands: Commands) {
    commands.insert_resource(LevelInstance::new());

    #[derive(Component, Default)]
    struct Ec3d;

    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    priority: 100,
                    is_active: true,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 2.0, 5.0),
                ..Camera3dBundle::default()
            },
            PickingCameraBundle::default(),
        ))
        .insert(Ec3d)
        .insert(camera_3d_free::FlycamControls::default())
        .insert(EditorCamera)
        .insert(Name::new("Editor Camera 3D Free"));
}

fn stop_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    current_level_asset_path: Res<CurrentLevelResourcePath>,
    asset_server: Res<AssetServer>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(LoadingLevel(asset_server.load(&current_level_asset_path.0)));
    commands.insert_resource(NextState(GameState::Game));
}

#[allow(clippy::too_many_arguments)]
fn add_wall_on_click_system(
    buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    windows: Res<Windows>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut level_instance: ResMut<LevelInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
    mut shapes: ResMut<DebugShapes>,
) {
    if !keyboard.pressed(KeyCode::LControl) || !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = windows.get_primary().unwrap();
    let Some(mouse_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = camera.single();
    let ray = ray_from_screen_space(mouse_position, camera, camera_transform);

    let mut mesh_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    if let Some(position) = level_instance.find_first_free_cell_on_ray(ray, shapes.as_mut()) {
        spawn_wall(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
            assets.as_ref(),
        );
    }
}

fn move_selected_grid_entity(
    keyboard: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut level_instance: ResMut<LevelInstance>,
    mut selection: Query<(&Selection, &mut GridEntity, &mut Transform)>,
    camera: Query<&GlobalTransform, With<EditorCamera>>,
) {
    if mouse_input.pressed(MouseButton::Right) {
        return;
    }

    let camera_transform = camera.single();
    let right = camera_transform.right();

    let horizonthal_directions = [Vec3::NEG_X, Vec3::X, Vec3::NEG_Z, Vec3::Z];
    let mut x_axis = horizonthal_directions[0];

    for direction in horizonthal_directions.iter().skip(1) {
        if x_axis.dot(right) < direction.dot(right) {
            x_axis = *direction;
        }
    }

    let x_axis = x_axis.as_ivec3();
    let z_axis = IVec3::new(-x_axis.z, 0, x_axis.x);

    let move_direction = if keyboard.just_pressed(KeyCode::W) {
        Some(-z_axis)
    } else if keyboard.just_pressed(KeyCode::A) {
        Some(-x_axis)
    } else if keyboard.just_pressed(KeyCode::S) {
        Some(z_axis)
    } else if keyboard.just_pressed(KeyCode::D) {
        Some(x_axis)
    } else if keyboard.just_pressed(KeyCode::Q) {
        Some(IVec3::NEG_Y)
    } else if keyboard.just_pressed(KeyCode::E) {
        Some(IVec3::Y)
    } else {
        None
    };

    let Some(direction) = move_direction else {
        return;
    };

    let mut moves = Vec::with_capacity(selection.iter().len());

    for (selection, mut grid_entity, mut transform) in &mut selection {
        if !selection.selected() {
            continue;
        }

        moves.push((
            grid_entity.0,
            grid_entity.0 + direction,
            *level_instance.get(grid_entity.0).unwrap(),
        ));

        grid_entity.0 += direction;
        transform.translation += direction.as_vec3();
    }

    for (old, _, _) in &moves {
        level_instance.set_empty(*old);
    }
    for (_, new, value) in moves {
        level_instance.mark_position_occupied(new, value);
    }
}

fn delete_selected_wall_system(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut level_instance: ResMut<LevelInstance>,
    selection: Query<(Entity, &GridEntity, &Selection)>,
) {
    if !keyboard.just_pressed(KeyCode::Back) {
        return;
    }

    for (entity, grid_entity, selection) in &selection {
        if !selection.selected() {
            continue;
        }

        level_instance.set_empty(grid_entity.0);
        commands.entity(entity).despawn();
    }
}

fn save_level_system(
    keyboard: Res<Input<KeyCode>>,
    snake_query: Query<&Snake>,
    walls_query: Query<&GridEntity, With<Wall>>,
    foods_query: Query<&GridEntity, With<Food>>,
    spikes_query: Query<&GridEntity, With<Spike>>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::S) {
        return;
    }

    let template = LevelTemplate {
        snakes: snake_query
            .iter()
            .map(|snake| snake.parts().clone().into())
            .collect(),
        foods: foods_query.into_iter().map(|entity| entity.0).collect(),
        spikes: spikes_query.into_iter().map(|entity| entity.0).collect(),
        walls: walls_query.into_iter().map(|entity| entity.0).collect(),
    };

    let ron_string = ron::ser::to_string_pretty(&template, PrettyConfig::default()).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            File::create("assets/level1.lvl")
                .and_then(|mut file| file.write(ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}
