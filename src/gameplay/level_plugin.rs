use bevy::prelude::*;

use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionHelpers, ConditionSet, IntoConditionalSystem},
    state::NextState,
};

use crate::{
    level::level_instance::LevelInstance,
    level::level_template::{LevelTemplateLoader, LoadedLevel},
    level::{
        level_template::{LevelTemplate, LoadingLevel},
        levels::*,
    },
    tools::cameras::camera_3d_free::FlycamControls,
    Assets, GameAssets, GameState,
};

use super::{
    commands::SnakeCommands,
    level_entities::*,
    movement_plugin::{GravityFall, SnakeReachGoalEvent},
    movement_plugin::{LevelExitAnim, MovementStages, SnakeExitedLevelEvent},
    snake_plugin::MaterialMeshBuilder,
    snake_plugin::{Active, SelectedSnake, Snake},
    undo::SnakeHistory,
};

pub struct StartLevelEventWithIndex(pub usize);
pub struct StartTestLevelEventWithIndex(pub usize);
pub struct StartLevelEventWithLevelAssetPath(pub String);
pub struct LevelLoadedEvent;
pub struct ClearLevelEvent;

#[derive(Resource)]
pub struct CurrentLevelId(pub usize);

#[derive(Resource)]
pub struct CurrentLevelAssetPath(pub String);

pub struct LevelPlugin;

#[derive(Component, Clone, Copy)]
pub struct Water;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel, StageLabel)]
pub enum LevelStages {
    LoadLevelStage,
    CheckLevelCondition,
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<LevelTemplate>()
            .init_asset_loader::<LevelTemplateLoader>()
            .add_exit_system(GameState::Game, clear_level_runtime_resources_system)
            .add_event::<StartLevelEventWithIndex>()
            .add_event::<StartTestLevelEventWithIndex>()
            .add_event::<StartLevelEventWithLevelAssetPath>()
            .add_event::<LevelLoadedEvent>()
            .add_event::<ClearLevelEvent>()
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .with_system(load_level_with_index_system)
                    .with_system(load_test_level_with_index_system)
                    .with_system(load_level_system)
                    .into(),
            )
            .add_system(
                notify_level_loaded_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LoadingLevel>()
                    .label(LevelStages::LoadLevelStage),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                spawn_level_entities_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LoadedLevel>()
                    .run_if_resource_exists::<LevelInstance>(),
            )
            // .add_system_to_stage(
            //     CoreStage::PostUpdate,
            //     activate_goal_when_all_food_eaten_system
            //         .run_in_state(GameState::Game)
            //         .run_if_resource_exists::<LevelInstance>()
            //         .label(LevelStages::CheckLevelCondition),
            // )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                activate_goal_when_trigger_pressed_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::SmoothMovement),
            )
            .add_system(
                start_snake_exit_level_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system(
                finish_snake_exit_level_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system_to_stage(
                CoreStage::Last,
                check_for_level_completion_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(LevelStages::CheckLevelCondition),
            )
            .add_system_to_stage(
                CoreStage::Last,
                clear_level_system.run_in_state(GameState::Game),
            );
    }
}

fn load_level_with_index_system(
    mut commands: Commands,
    mut event_start_level_with_index: EventReader<StartLevelEventWithIndex>,
    mut event_start_level: EventWriter<StartLevelEventWithLevelAssetPath>,
) {
    let Some(event) = event_start_level_with_index.iter().next() else {
        return;
    };

    let next_level_index = event.0;
    let level_asset_path = format!("levels/{}", LEVELS[next_level_index]);

    event_start_level.send(StartLevelEventWithLevelAssetPath(level_asset_path.clone()));

    commands.insert_resource(CurrentLevelId(next_level_index));
    commands.insert_resource(CurrentLevelAssetPath(level_asset_path));
}

fn load_test_level_with_index_system(
    mut commands: Commands,
    mut event_start_level_with_index: EventReader<StartTestLevelEventWithIndex>,
    mut event_start_level: EventWriter<StartLevelEventWithLevelAssetPath>,
) {
    let Some(event) = event_start_level_with_index.iter().next() else {
        return;
    };

    let next_level_index = event.0;
    let level_asset_path = format!("test_levels/{}", TEST_LEVELS[next_level_index]);

    event_start_level.send(StartLevelEventWithLevelAssetPath(level_asset_path.clone()));

    commands.insert_resource(CurrentLevelId(next_level_index));
    commands.insert_resource(CurrentLevelAssetPath(level_asset_path));
}

pub fn load_level_system(
    mut commands: Commands,
    mut event_start_level: EventReader<StartLevelEventWithLevelAssetPath>,
    asset_server: Res<AssetServer>,
) {
    let Some(event) = event_start_level.iter().next() else {
        return;
    };

    let template: Handle<LevelTemplate> = asset_server.load(&event.0);
    commands.insert_resource(LoadingLevel(template));
}

fn notify_level_loaded_system(
    mut commands: Commands,
    level_loading: Res<LoadingLevel>,
    asset_server: Res<AssetServer>,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
) {
    let load_state = asset_server.get_load_state(&level_loading.0);
    match load_state {
        bevy::asset::LoadState::Loaded => {
            commands.remove_resource::<LoadingLevel>();

            commands.insert_resource(LoadedLevel(level_loading.0.clone()));
            commands.insert_resource(SnakeHistory::default());
            commands.insert_resource(LevelInstance::new());

            level_loaded_event.send(LevelLoadedEvent);
        }
        bevy::asset::LoadState::Failed => panic!("Failed loading level"),
        _ => {}
    }
}

pub fn clear_level_runtime_resources_system(mut commands: Commands) {
    commands.remove_resource::<LevelInstance>();
    commands.remove_resource::<SnakeHistory>();
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_level_entities_system(
    level_loaded_event: EventReader<LevelLoadedEvent>,
    mut commands: Commands,
    mut level_instance: ResMut<LevelInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
    loaded_level: Res<LoadedLevel>,
    level_templates: ResMut<Assets<LevelTemplate>>,
    mut camera: Query<(&mut Transform, Option<&mut FlycamControls>), With<Camera>>,
) {
    if level_loaded_event.is_empty() {
        return;
    }
    level_loaded_event.clear();

    let level_template = level_templates
        .get(&loaded_level.0)
        .expect("Level should be loaded here!");

    let mut min = 1000 * IVec3::ONE;
    let mut max = 1000 * IVec3::NEG_ONE;

    level_template.walls.iter().for_each(|position| {
        min = min.min(*position);
        max = max.max(*position);
    });

    let center = min.as_vec3() + 0.5 * (max - min).as_vec3();

    let (mut camera_transform, fly_camera) = camera.single_mut();
    *camera_transform = Transform::from_translation(center + 15.0 * Vec3::Y + 12.0 * Vec3::Z)
        .looking_at(center, Vec3::Y);

    if let Some(mut fly_camera) = fly_camera {
        fly_camera.set_transform(&camera_transform);
    }

    // light
    let size = 25.0;
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::rgb(1.0, 1.0, 1.0),
                illuminance: 10000.0,
                shadows_enabled: true,
                shadow_projection: OrthographicProjection {
                    left: -size,
                    right: size,
                    bottom: -size,
                    top: size,
                    near: -size,
                    far: size,
                    ..Default::default()
                },
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.5, 3.0, 0.5))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        LevelEntity,
    ));

    let mut mesh_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    // Spawn the wall blocks
    for position in &level_template.walls {
        spawn_wall(
            &mut mesh_builder,
            &mut commands,
            position,
            &mut level_instance,
            assets.as_ref(),
        );
    }

    // Spawn the food sprites.
    for position in &level_template.foods {
        spawn_food(
            &mut mesh_builder,
            &mut commands,
            position,
            &mut level_instance,
        );
    }

    // Spawn the spikes sprites.
    for position in &level_template.spikes {
        spawn_spike(
            &mut mesh_builder,
            &mut commands,
            position,
            &mut level_instance,
        );
    }

    for position in &level_template.boxes {
        spawn_box(
            &mut mesh_builder,
            &mut commands,
            position,
            &mut level_instance,
        );
    }

    for position in &level_template.triggers {
        spawn_trigger(
            &mut mesh_builder,
            &mut commands,
            position,
            &mut level_instance,
        );
    }

    if let Some(goal_position) = level_template.goal {
        spawn_goal(&mut commands, &goal_position, &mut level_instance, &assets);
    };

    for (snake_index, snake_template) in level_template.snakes.iter().enumerate() {
        let entity = spawn_snake(
            &mut mesh_builder,
            &mut commands,
            &mut level_instance,
            snake_template,
            snake_index as i32,
        );

        if snake_index == 0 {
            commands.entity(entity).insert(SelectedSnake);
        }
    }
}

pub fn clear_level_system(
    mut event_clear_level: EventReader<ClearLevelEvent>,
    mut commands: Commands,
    query: Query<Entity, (With<LevelEntity>, Without<Camera>)>,
) {
    if event_clear_level.iter().next().is_none() {
        return;
    }

    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<LevelInstance>();
    commands.remove_resource::<SnakeHistory>();
}

fn _activate_goal_when_all_food_eaten_system(
    mut commands: Commands,
    food_query: Query<&Food>,
    goal_query: Query<(Entity, Option<&Active>), With<Goal>>,
) {
    let Ok((goal_entity, active)) = goal_query.get_single() else {
        return;
    };

    if food_query.is_empty() {
        if active.is_none() {
            commands.entity(goal_entity).insert(Active);
        }
    } else if active.is_some() {
        commands.entity(goal_entity).remove::<Active>();
    }
}

#[allow(clippy::type_complexity)]
fn activate_goal_when_trigger_pressed_system(
    mut commands: Commands,
    triggers_query: Query<&Trigger, Without<Active>>,
    assets: Res<GameAssets>,
    mut goal_query: Query<
        (
            Entity,
            Option<&Active>,
            &mut Handle<Mesh>,
            &mut Handle<StandardMaterial>,
        ),
        With<Goal>,
    >,
) {
    let Ok((goal_entity, active, mut mesh, mut material)) = goal_query.get_single_mut() else {
        return;
    };

    if triggers_query.is_empty() {
        if active.is_none() {
            commands.entity(goal_entity).insert(Active);
            *mesh = assets.goal_active_mesh.clone();
            *material = assets.goal_active_material.clone();
        }
    } else if active.is_some() {
        commands.entity(goal_entity).remove::<Active>();
        *mesh = assets.goal_inactive_mesh.clone();
        *material = assets.goal_inactive_material.clone();
    }
}

#[allow(clippy::type_complexity)]
pub fn check_for_level_completion_system(
    mut snake_reach_goal_event: EventWriter<SnakeReachGoalEvent>,
    snakes_query: Query<(Entity, &Snake), (With<Active>, Without<LevelExitAnim>)>,
    goal_query: Query<&GridEntity, (With<Goal>, With<Active>)>,
) {
    let Ok(goal) = goal_query.get_single() else {
        return;
    };

    let snake_at_exit = snakes_query
        .iter()
        .find(|(_, snake)| goal.0 == snake.head_position());
    if snake_at_exit.is_none() {
        return;
    }

    snake_reach_goal_event.send(SnakeReachGoalEvent(snake_at_exit.unwrap().0));
}

#[allow(clippy::type_complexity)]
pub fn start_snake_exit_level_system(
    mut history: ResMut<SnakeHistory>,
    mut level_instance: ResMut<LevelInstance>,
    mut snake_reach_goal_event: EventReader<SnakeReachGoalEvent>,
    mut commands: Commands,
    snakes_query: Query<
        (Entity, &Snake, Option<&GravityFall>, Option<&SelectedSnake>),
        With<Active>,
    >,
) {
    if let Some(reach_goal_event) = snake_reach_goal_event.iter().next() {
        let (entity, snake, gravity, selected_snake) = snakes_query
            .get(reach_goal_event.0)
            .expect("Snake should be in query.");

        commands
            .entity(entity)
            .remove::<SelectedSnake>()
            .remove::<GravityFall>();

        SnakeCommands::new(level_instance.as_mut(), history.as_mut())
            .exit_level(snake, entity, gravity);

        // Select another snake if the snake was selected.
        if selected_snake.is_some() {
            let other_snake = snakes_query
                .iter()
                .find(|(other_entity, _, _, _)| entity != *other_entity);

            if let Some((next_snake_entity, _, _, _)) = other_snake {
                commands.entity(next_snake_entity).insert(SelectedSnake);
            }
        }

        // Start anim
        commands.entity(entity).insert(LevelExitAnim {
            distance_to_move: snake.len() as i32,
            initial_snake_position: snake.parts().clone().into(),
        });
    }

    snake_reach_goal_event.clear();
}

pub fn finish_snake_exit_level_system(
    mut commands: Commands,
    level_id: Res<CurrentLevelId>,
    snake_reach_goal_event: EventReader<SnakeExitedLevelEvent>,
    mut event_start_level: EventWriter<StartLevelEventWithIndex>,
    mut event_clear_level: EventWriter<ClearLevelEvent>,
    snakes_query: Query<&Snake, With<Active>>,
) {
    if snake_reach_goal_event.is_empty() {
        return;
    }

    if snakes_query.is_empty() {
        if level_id.0 == LEVELS.len() - 1 {
            event_clear_level.send(ClearLevelEvent);
            commands.insert_resource(NextState(GameState::MainMenu));
        } else {
            event_clear_level.send(ClearLevelEvent);
            event_start_level.send(StartLevelEventWithIndex(level_id.0 + 1));
        }
    }
}
