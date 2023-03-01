use std::{fs::File, io::Write};

use bevy::{gltf::Gltf, prelude::*, tasks::IoTaskPool};
use bevy_inspector_egui::{prelude::ReflectInspectorOptions, InspectorOptions};
use bevy_prototype_debug_lines::DebugLinesMesh;
use bevy_reflect::Reflect;
use egui::Align2;
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
            CurrentLevelMetadata, LevelLoadedEvent,
        },
        snake_plugin::{
            despawn_snake_part_system, update_snake_transforms_system, DespawnSnakePartEvent,
            MaterialMeshBuilder, Snake, SnakePart,
        },
    },
    level::{
        level_instance::{LevelGridEntity, LevelInstance},
        level_template::{EntityTemplate, LevelTemplate, LoadedLevel, LoadingLevel, Model},
    },
    library::GameAssets,
    tools::{
        cameras::{camera_3d_free, EditorCamera},
        picking::PickingCameraBundle,
    },
    utils::ray_from_screen_space,
    GameState,
};

use super::{
    cameras::camera_3d_free::FlycamPlugin,
    picking::{DefaultPickingPlugins, PickableBundle, PickableMesh, Selection},
};

pub struct EditorPlugin;

#[derive(Resource, InspectorOptions, Reflect)]
#[reflect(InspectorOptions)]
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
                    .run_unless_resource_exists::<CurrentLevelMetadata>(),
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
                    .with_system(move_selected_snake_system)
                    .with_system(resize_selected_snake_system)
                    .with_system(despawn_snake_part_system)
                    .with_system(ui_editor)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<CurrentLevelMetadata>()
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
    level_meta: Res<CurrentLevelMetadata>,
    asset_server: Res<AssetServer>,
    editor_camera: Query<Entity, With<EditorCamera>>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(ResumeFromEditor);
    commands.insert_resource(LoadingLevel(asset_server.load(&level_meta.asset_path)));
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
        EntityType::Food => spawn_food(&mut mesh_builder, &mut commands, &position),
        EntityType::Spike => spawn_spike(&mut mesh_builder, &mut commands, &position),
        EntityType::Wall => {
            spawn_wall(&mut mesh_builder, &mut commands, &position, assets.as_ref())
        }
        EntityType::Box => spawn_box(&mut mesh_builder, &mut commands, &position),
        EntityType::Trigger => spawn_trigger(&mut mesh_builder, &mut commands, &position),
        EntityType::Snake => spawn_snake(
            &mut mesh_builder,
            &mut commands,
            &mut level_instance,
            &vec![(position, IVec3::X), (position - IVec3::X, IVec3::X)],
            snakes.iter().len() as i32,
        ),
        EntityType::Goal => spawn_goal(&mut commands, &position, &assets, &gltfs),
    };

    level_instance.mark_position_occupied(
        position,
        LevelGridEntity::new(id, editor_state.insert_entity_type),
    );

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
            grid_entity.position,
            grid_entity.position + direction,
            *level_instance.get(grid_entity.position).unwrap(),
        ));

        grid_entity.position += direction;
        transform.translation += direction.as_vec3();
    }

    for (old, _, _) in &moves {
        level_instance.set_empty(*old);
    }

    for (_, new, value) in moves {
        level_instance.mark_position_occupied(new, value);
    }
}

fn move_selected_snake_system(
    keyboard: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut level_instance: ResMut<LevelInstance>,
    mut selection: Query<(Entity, &Selection, &mut Snake, &mut Transform)>,
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

    for (entity, selection, mut snake, mut transform) in &mut selection {
        if !selection.selected() || direction == -snake.head_direction() {
            continue;
        }

        for position in snake.positions() {
            moves.push((
                *position,
                *position + direction,
                LevelGridEntity::new(entity, EntityType::Snake),
            ));
        }

        snake.move_forward(direction);
        transform.translation += direction.as_vec3();
    }

    for (old, _, _) in &moves {
        level_instance.set_empty(*old);
    }

    for (_, new, value) in moves {
        level_instance.mark_position_occupied(new, value);
    }
}

fn resize_selected_snake_system(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut level_instance: ResMut<LevelInstance>,
    mut selection: Query<(Entity, &Selection, &mut Snake)>,
    mut despawn_snake_part: EventWriter<DespawnSnakePartEvent>,
) {
    let mut mesh_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    for (entity, selection, mut snake) in &mut selection {
        if !selection.selected() {
            continue;
        }

        if keyboard.just_pressed(KeyCode::Equals) {
            snake.grow();

            commands.entity(entity).with_children(|parent| {
                let part_id = parent
                    .spawn(mesh_builder.build_part(
                        snake.tail_position(),
                        snake.index(),
                        snake.len() - 1,
                    ))
                    .id();
                level_instance.mark_position_occupied(
                    snake.tail_position(),
                    LevelGridEntity::new(part_id, EntityType::Snake),
                );
            });
        } else if keyboard.just_pressed(KeyCode::Minus) {
            if snake.len() <= 2 {
                continue;
            }

            level_instance.set_empty(snake.tail_position());
            despawn_snake_part.send(DespawnSnakePartEvent(SnakePart {
                snake_index: snake.index(),
                part_index: snake.len() - 1,
            }));

            snake.shrink();
        }
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

        level_instance.set_empty(grid_entity.position);
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

    let new_tempale = LevelTemplate {
        entities: walls
            .into_iter()
            .map(|position| EntityTemplate {
                entity_type: EntityType::Wall,
                model: Model::Default(EntityType::Wall.into()),
                grid_position: position,
            })
            .collect(),
        ..default()
    };

    commands.insert_resource(CurrentLevelMetadata {
        id: None,
        asset_path: "levels/new.lvl".to_owned(),
    });
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
    level_meta: Res<CurrentLevelMetadata>,
    snake_query: Query<&Snake>,
    entities: Query<&GridEntity>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::S) {
        return;
    }

    let template = LevelTemplate {
        snakes: snake_query
            .iter()
            .map(|snake| snake.parts().clone().into())
            .collect(),
        entities: entities
            .into_iter()
            .map(|entity| EntityTemplate {
                entity_type: entity.entity_type,
                model: Model::Default(entity.entity_type.into()),
                grid_position: entity.position,
            })
            .collect(),
    };

    let ron_string = ron::ser::to_string_pretty(&template, PrettyConfig::default()).unwrap();
    let level_asset_path = level_meta.asset_path.clone();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            File::create(format!("assets/{}", level_asset_path))
                .and_then(|mut file| file.write(ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

fn ui_editor(world: &mut World) {
    let egui_context = world
        .resource_mut::<bevy_egui::EguiContext>()
        .ctx_mut()
        .clone();

//     egui::Area::new("my_area")
//         .pivot(Align2::RIGHT_TOP)
//         .fixed_pos(egui::pos2(1080.0, 0.0))
//         .show(&egui_context, |ui| {
//             bevy_inspector_egui::bevy_inspector::ui_for_resource::<EditorState>(world, ui);
//         });
// }
