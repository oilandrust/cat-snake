use std::{fs::File, io::Write};

use bevy::{gltf::Gltf, prelude::*, tasks::IoTaskPool};
use bevy_prototype_debug_lines::DebugLinesMesh;
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem},
    state::NextState,
};
use ron::ser::PrettyConfig;

use crate::{
    despawn_entities, despawn_with_system,
    gameplay::{
        level_entities::*,
        level_plugin::{
            clear_level_runtime_resources_system, spawn_level_entities_system,
            CurrentLevelAssetPath, LevelLoadedEvent,
        },
        snake_plugin::{update_snake_transforms_system, MaterialMeshBuilder, Snake},
    },
    level::{
        level_instance::{EntityType, LevelInstance},
        level_template::{LevelTemplate, LoadedLevel, LoadingLevel},
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
    picking::{DefaultPickingPlugins, PickableBundle, PickableMesh, Selection},
};

pub struct EditorPlugin;

#[derive(Resource)]
struct EditorState {
    insert_entity_type: EntityType,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            insert_entity_type: EntityType::Wall,
        }
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(FlycamPlugin)
            .insert_resource(EditorState::default())
            .add_system(start_editor_system.run_in_state(GameState::Game))
            .add_enter_system(GameState::Editor, init_level_instance_system)
            .add_enter_system(
                GameState::Editor,
                create_new_level_on_enter_system
                    .run_unless_resource_exists::<CurrentLevelAssetPath>(),
            )
            .add_exit_system(GameState::Editor, despawn_with_system::<LevelEntity>)
            .add_exit_system(GameState::Editor, clear_level_runtime_resources_system)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>()
                    .with_system(spawn_level_entities_system)
                    .with_system(bevy::window::close_on_esc)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .with_system(choose_entity_to_add_system)
                    .with_system(add_pickable_to_level_entities_system)
                    .with_system(add_entity_on_click_system)
                    .with_system(select_parent_level_entity_system)
                    .with_system(delete_selected_entity_system)
                    .with_system(create_new_level_system)
                    .with_system(update_snake_transforms_system)
                    .with_system(move_selected_grid_entity)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<CurrentLevelAssetPath>()
                    .with_system(save_level_system)
                    .with_system(stop_editor_system)
                    .into(),
            );
    }
}

fn start_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(NextState(GameState::Editor));
    level_loaded_event.send(LevelLoadedEvent);
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
                transform: Transform::from_translation(15.0 * Vec3::Y + 12.0 * Vec3::Z)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..Camera3dBundle::default()
            },
            PickingCameraBundle::default(),
        ))
        .insert(Ec3d)
        .insert(camera_3d_free::FlycamControls::default())
        .insert(EditorCamera)
        .insert(Name::new("Editor Camera 3D Free"));
}

#[derive(Resource, Default)]
pub struct ResumeFromEditor;

fn stop_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    current_level_asset_path: Res<CurrentLevelAssetPath>,
    asset_server: Res<AssetServer>,
    editor_camera: Query<Entity, With<EditorCamera>>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(ResumeFromEditor);
    commands.insert_resource(LoadingLevel(asset_server.load(&current_level_asset_path.0)));
    commands.insert_resource(NextState(GameState::Game));
    commands.entity(editor_camera.single()).despawn();
}

fn choose_entity_to_add_system(
    keyboard: Res<Input<KeyCode>>,
    mut editor_state: ResMut<EditorState>,
) {
    if keyboard.just_pressed(KeyCode::G) {
        editor_state.insert_entity_type = EntityType::Goal;
    } else if keyboard.just_pressed(KeyCode::F) {
        editor_state.insert_entity_type = EntityType::Food;
    } else if keyboard.just_pressed(KeyCode::H) {
        editor_state.insert_entity_type = EntityType::Wall;
    } else if keyboard.just_pressed(KeyCode::K) {
        editor_state.insert_entity_type = EntityType::Spike;
    } else if keyboard.just_pressed(KeyCode::B) {
        editor_state.insert_entity_type = EntityType::Box;
    } else if keyboard.just_pressed(KeyCode::T) {
        editor_state.insert_entity_type = EntityType::Trigger;
    } else if keyboard.just_pressed(KeyCode::L) {
        editor_state.insert_entity_type = EntityType::Snake;
    }
}

#[allow(clippy::too_many_arguments)]
fn add_entity_on_click_system(
    buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    editor_state: Res<EditorState>,
    windows: Res<Windows>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut level_instance: ResMut<LevelInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gltfs: Res<Assets<Gltf>>,
    snakes: Query<&Snake>,
    assets: Res<GameAssets>,
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

    let Some(position) = level_instance.find_first_free_cell_on_ray(ray) else {
        return;
    };

    let id = match editor_state.insert_entity_type {
        EntityType::Food => spawn_food(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
        ),
        EntityType::Spike => spawn_spike(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
        ),
        EntityType::Wall => spawn_wall(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
            assets.as_ref(),
        ),
        EntityType::Box => spawn_box(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
        ),
        EntityType::Trigger => spawn_trigger(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
        ),
        EntityType::Snake => spawn_snake(
            &mut mesh_builder,
            &mut commands,
            &mut level_instance,
            &vec![(position, IVec3::X), (position - IVec3::X, IVec3::X)],
            snakes.iter().len() as i32,
        ),
        EntityType::Goal => spawn_goal(
            &mut commands,
            &position,
            &mut level_instance,
            &assets,
            &gltfs,
        ),
    };

    commands.entity(id).insert(PickableBundle::default());
}

#[allow(clippy::type_complexity)]
fn add_pickable_to_level_entities_system(
    mut commands: Commands,
    grid_entities: Query<
        Entity,
        (
            Or<(With<Handle<Mesh>>, With<LevelEntity>)>,
            Without<PickableMesh>,
            Without<DebugLinesMesh>,
        ),
    >,
) {
    for entity in &grid_entities {
        commands.entity(entity).insert(PickableBundle::default());
    }
}

#[allow(clippy::type_complexity)]
fn select_parent_level_entity_system(
    changed_selection: Query<(Entity, &Selection), (Changed<Selection>, Without<LevelEntity>)>,
    parents: Query<&Parent>,
    mut level_entitites: Query<&mut Selection, With<LevelEntity>>,
) {
    for (entity, selection) in &changed_selection {
        if !selection.selected() {
            continue;
        }

        let mut current_parent = parents.get(entity).ok();
        while let Some(parent) = current_parent {
            if let Ok(mut selection) = level_entitites.get_mut(parent.get()) {
                selection.set_selected(true);
                break;
            }

            current_parent = parents.get(parent.get()).ok();
        }
    }
}

fn select_move_direction(
    keyboard: &Input<KeyCode>,
    camera_transform: &GlobalTransform,
) -> Option<IVec3> {
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

    if keyboard.just_pressed(KeyCode::W) {
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
    }
}

fn move_selected_grid_entity(
    keyboard: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut level_instance: ResMut<LevelInstance>,
    mut selection: Query<(&Selection, &mut GridEntity, &mut Transform)>,
    camera: Query<&GlobalTransform, With<EditorCamera>>,
) {
    if mouse_input.pressed(MouseButton::Right) || !keyboard.pressed(KeyCode::LControl) {
        return;
    }

    let camera_transform = camera.single();
    let Some(direction) = select_move_direction(&keyboard, camera_transform) else {
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

fn delete_selected_entity_system(
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

fn create_new_level(
    level_loaded_event: &mut EventWriter<LevelLoadedEvent>,
    levels: &mut Assets<LevelTemplate>,
    commands: &mut Commands,
) {
    let mut walls = Vec::with_capacity(10 * 10);
    for j in -5..5 {
        for i in -5..5 {
            walls.push(IVec3::new(i, 0, j));
        }
    }

    let new_tempale = LevelTemplate { walls, ..default() };

    commands.insert_resource(CurrentLevelAssetPath("levels/new.lvl".to_owned()));
    commands.insert_resource(LoadedLevel(levels.add(new_tempale)));
    commands.insert_resource(LevelInstance::new());
    level_loaded_event.send(LevelLoadedEvent);
}

fn create_new_level_on_enter_system(
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
    mut levels: ResMut<Assets<LevelTemplate>>,
    mut commands: Commands,
) {
    create_new_level(&mut level_loaded_event, &mut levels, &mut commands);
}

fn create_new_level_system(
    keyboard: Res<Input<KeyCode>>,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
    mut levels: ResMut<Assets<LevelTemplate>>,
    mut commands: Commands,
    entities: Query<Entity, With<LevelEntity>>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::N) {
        return;
    }
    despawn_entities::<LevelEntity>(&mut commands, entities);

    create_new_level(&mut level_loaded_event, &mut levels, &mut commands);
}

#[allow(clippy::too_many_arguments)]
fn save_level_system(
    keyboard: Res<Input<KeyCode>>,
    current_level_asset_path: Res<CurrentLevelAssetPath>,
    snake_query: Query<&Snake>,
    walls_query: Query<&GridEntity, With<Wall>>,
    foods_query: Query<&GridEntity, With<Food>>,
    spikes_query: Query<&GridEntity, With<Spike>>,
    boxes_query: Query<&GridEntity, With<Box>>,
    triggers_query: Query<&GridEntity, With<Trigger>>,
    goal_query: Query<&GridEntity, With<Goal>>,
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
        boxes: boxes_query.into_iter().map(|entity| entity.0).collect(),
        triggers: triggers_query.into_iter().map(|entity| entity.0).collect(),
        goal: goal_query.get_single().map(|entity| entity.0).ok(),
    };

    let ron_string = ron::ser::to_string_pretty(&template, PrettyConfig::default()).unwrap();
    let level_asset_path = current_level_asset_path.0.clone();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            File::create(format!("assets/{}", level_asset_path))
                .and_then(|mut file| file.write(ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}
