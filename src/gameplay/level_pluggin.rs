use bevy::{app::AppExit, prelude::*};

use iyes_loopless::prelude::{
    AppLooplessStateExt, ConditionHelpers, ConditionSet, IntoConditionalSystem,
};

use crate::{
    gameplay::commands::SnakeCommands,
    gameplay::movement_pluggin::{GravityFall, SnakeReachGoalEvent},
    gameplay::snake_pluggin::{Active, SelectedSnake, Snake, SpawnSnakeEvent},
    gameplay::undo::SnakeHistory,
    level::level_instance::{LevelEntityType, LevelInstance},
    level::{
        level_template::{LevelTemplate, LoadingLevel},
        levels::LEVELS,
    },
    level::{
        level_template::{LevelTemplateLoader, LoadedLevel},
        test_levels::TEST_LEVELS,
    },
    tools::picking::{PickableBundle, PickingCameraBundle},
    Assets, GameAssets, GameState,
};

use super::{
    game_constants_pluggin::{FOOD_COLOR, SPIKE_COLOR},
    movement_pluggin::{LevelExitAnim, SnakeExitedLevelEvent},
    snake_pluggin::MaterialMeshBuilder,
};

pub struct StartLevelEventWithIndex(pub usize);
pub struct StartTestLevelEventWithIndex(pub usize);
pub struct StartLevelEventWithLevelAssetPath(pub String);
pub struct LevelLoadedEvent;
pub struct ClearLevelEvent;

#[derive(Component, Reflect)]
pub struct LevelEntity;

#[derive(Component, Clone, Copy)]
pub struct GridEntity(pub IVec3);

#[derive(Component, Clone, Copy)]
pub struct Wall;

#[derive(Component, Clone, Copy)]
pub struct Food;

#[derive(Component, Clone, Copy)]
pub struct Spike;

#[derive(Component, Clone, Copy)]
pub struct Goal;

#[derive(Resource)]
pub struct CurrentLevelId(pub usize);

#[derive(Resource)]
pub struct CurrentLevelResourcePath(pub String);

pub struct LevelPluggin;

#[derive(Component, Clone, Copy)]
pub struct Water;

pub static LOAD_LEVEL_STAGE: &str = "LoadLevelStage";
static PRE_LOAD_LEVEL_LABEL: &str = "PreloadLevel";
static CHEK_LEVEL_CONDITION_LABEL: &str = "CheckLevelCondition";

impl Plugin for LevelPluggin {
    fn build(&self, app: &mut App) {
        app.add_asset::<LevelTemplate>()
            .init_asset_loader::<LevelTemplateLoader>()
            .add_exit_system(GameState::Game, clear_level_runtime_resources_system)
            .add_event::<StartLevelEventWithIndex>()
            .add_event::<StartTestLevelEventWithIndex>()
            .add_event::<StartLevelEventWithLevelAssetPath>()
            .add_event::<LevelLoadedEvent>()
            .add_event::<ClearLevelEvent>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .with_system(load_level_with_index_system)
                    .with_system(load_test_level_with_index_system)
                    .with_system(load_level_system)
                    .into(),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                notify_level_loaded_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LoadingLevel>()
                    .label(PRE_LOAD_LEVEL_LABEL),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                spawn_level_entities_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .after(PRE_LOAD_LEVEL_LABEL),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                activate_goal_when_all_food_eaten_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(CHEK_LEVEL_CONDITION_LABEL),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                check_for_level_completion_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(CHEK_LEVEL_CONDITION_LABEL),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                start_snake_exit_level_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .after(CHEK_LEVEL_CONDITION_LABEL),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                finish_snake_exit_level_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system_to_stage(
                CoreStage::Last,
                clear_level_system.run_in_state(GameState::Game),
            )
            .add_system(rotate_goal_system.run_in_state(GameState::Game));
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
    event_start_level.send(StartLevelEventWithLevelAssetPath(
        LEVELS[next_level_index].to_owned(),
    ));

    commands.insert_resource(CurrentLevelId(next_level_index));
    commands.insert_resource(CurrentLevelResourcePath(
        LEVELS[next_level_index].to_owned(),
    ));
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
    event_start_level.send(StartLevelEventWithLevelAssetPath(
        TEST_LEVELS[next_level_index].to_owned(),
    ));

    commands.insert_resource(CurrentLevelId(next_level_index));
    commands.insert_resource(CurrentLevelResourcePath(
        TEST_LEVELS[next_level_index].to_owned(),
    ));
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
    mut spawn_snake_event: EventWriter<SpawnSnakeEvent>,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
) {
    let load_state = asset_server.get_load_state(&level_loading.0);
    match load_state {
        bevy::asset::LoadState::Loaded => {
            commands.remove_resource::<LoadingLevel>();

            commands.insert_resource(LoadedLevel(level_loading.0.clone()));
            commands.insert_resource(SnakeHistory::default());
            commands.insert_resource(LevelInstance::new());

            spawn_snake_event.send(SpawnSnakeEvent);
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
) {
    if level_loaded_event.is_empty() {
        return;
    }
    level_loaded_event.clear();

    let level_template = level_templates
        .get(&loaded_level.0)
        .expect("Level should be loaded here!");

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::ZERO + 10.0 * Vec3::Y + 5.0 * Vec3::Z)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        LevelEntity,
    ));

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
}

pub fn spawn_spike(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    commands.spawn((
        mesh_builder.build_spike_mesh(*position),
        GridEntity(*position),
        Spike,
        LevelEntity,
    ));

    level_instance.mark_position_occupied(*position, LevelEntityType::Spike);
}

impl<'a> MaterialMeshBuilder<'a> {
    pub fn build_food_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Icosphere {
                radius: 0.3,
                subdivisions: 5,
            })),
            material: self.materials.add(FOOD_COLOR.into()),
            transform: Transform::from_translation(position.as_vec3()),
            ..default()
        }
    }

    pub fn build_spike_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
            material: self.materials.add(SPIKE_COLOR.into()),
            transform: Transform::from_translation(position.as_vec3()),
            ..default()
        }
    }
}

pub fn spawn_wall(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
    assets: &GameAssets,
) {
    let ground_material = mesh_builder.materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        base_color_texture: Some(assets.outline_texture.clone()),
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: mesh_builder
                .meshes
                .add(Mesh::from(shape::Cube { size: 1.0 })),
            material: ground_material,
            transform: Transform::from_translation(position.as_vec3()),
            ..default()
        },
        LevelEntity,
        GridEntity(*position),
        Wall,
        PickableBundle::default(),
    ));

    level_instance.mark_position_occupied(*position, LevelEntityType::Wall);
}

pub fn spawn_food(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    commands.spawn((
        mesh_builder.build_food_mesh(*position),
        GridEntity(*position),
        Food,
        LevelEntity,
        PickableBundle::default(),
    ));

    level_instance.mark_position_occupied(*position, LevelEntityType::Food);
}

pub fn clear_level_system(
    mut event_clear_level: EventReader<ClearLevelEvent>,
    mut commands: Commands,
    query: Query<Entity, With<LevelEntity>>,
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

fn activate_goal_when_all_food_eaten_system(
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

fn rotate_goal_system(
    time: Res<Time>,
    mut goal_query: Query<(&mut Transform, Option<&Active>), With<Goal>>,
) {
    let Ok((mut transform, active)) = goal_query.get_single_mut() else {
        return;
    };

    if active.is_some() {
        transform.rotate_local_z(time.delta_seconds() * 0.7);
        transform.scale = (1.6 + 0.3 * (time.elapsed_seconds() * 1.0).sin()) * Vec3::ONE;
    } else {
        transform.rotate_local_z(time.delta_seconds() * 0.3);
        transform.scale = Vec3::ONE;
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
    level_id: Res<CurrentLevelId>,
    snake_reach_goal_event: EventReader<SnakeExitedLevelEvent>,
    mut event_start_level: EventWriter<StartLevelEventWithIndex>,
    mut event_clear_level: EventWriter<ClearLevelEvent>,
    mut exit: EventWriter<AppExit>,
    snakes_query: Query<&Snake, With<Active>>,
) {
    if snake_reach_goal_event.is_empty() {
        return;
    }

    if snakes_query.is_empty() {
        if level_id.0 == LEVELS.len() - 1 {
            exit.send(AppExit);
        } else {
            event_clear_level.send(ClearLevelEvent);
            event_start_level.send(StartLevelEventWithIndex(level_id.0 + 1));
        }
    }
}
