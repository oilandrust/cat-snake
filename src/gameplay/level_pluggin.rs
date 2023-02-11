use std::{fs::File, io::Write};

use bevy::{
    app::AppExit,
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    tasks::IoTaskPool,
    utils::BoxedFuture,
};

use iyes_loopless::prelude::{ConditionHelpers, IntoConditionalSystem};
use ron::ser::PrettyConfig;

use crate::{
    gameplay::commands::SnakeCommands,
    gameplay::movement_pluggin::{GravityFall, SnakeReachGoalEvent},
    gameplay::snake_pluggin::{Active, SelectedSnake, Snake, SpawnSnakeEvent},
    gameplay::undo::SnakeHistory,
    level::level_instance::{LevelEntityType, LevelInstance},
    level::levels::LEVELS,
    level::test_levels::TEST_LEVELS,
    Assets, GameAssets, GameState,
};

use super::{
    game_constants_pluggin::{FOOD_COLOR, SPIKE_COLOR},
    movement_pluggin::{LevelExitAnim, SnakeExitedLevelEvent},
    snake_pluggin::{MaterialMeshBuilder, SnakeTemplate},
};

use serde::{Deserialize, Serialize};

pub struct StartLevelEventWithIndex(pub usize);
pub struct StartTestLevelEventWithIndex(pub usize);
pub struct StartLevelEventWithLevelAssetPath(pub String);
pub struct LevelLoadedEvent;
pub struct ClearLevelEvent;

#[derive(Component, Reflect)]
pub struct LevelEntity;

#[derive(Component, Clone, Copy)]
pub struct Food(pub IVec3);

#[derive(Component, Clone, Copy)]
pub struct Spike(pub IVec3);

#[derive(Component, Clone, Copy)]
pub struct Goal(pub IVec3);

#[derive(Resource)]
pub struct CurrentLevelId(pub usize);

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
            .add_event::<StartLevelEventWithIndex>()
            .add_event::<StartTestLevelEventWithIndex>()
            .add_event::<StartLevelEventWithLevelAssetPath>()
            .add_event::<LevelLoadedEvent>()
            .add_event::<ClearLevelEvent>()
            .add_stage_before(
                CoreStage::PreUpdate,
                LOAD_LEVEL_STAGE,
                SystemStage::single_threaded(),
            )
            .add_system_to_stage(
                LOAD_LEVEL_STAGE,
                load_level_with_index_system
                    .run_in_state(GameState::Game)
                    .label(PRE_LOAD_LEVEL_LABEL),
            )
            .add_system_to_stage(
                LOAD_LEVEL_STAGE,
                load_test_level_with_index_system
                    .run_in_state(GameState::Game)
                    .label(PRE_LOAD_LEVEL_LABEL),
            )
            .add_system_to_stage(
                LOAD_LEVEL_STAGE,
                load_level_system
                    .run_in_state(GameState::Game)
                    .after(PRE_LOAD_LEVEL_LABEL),
            )
            .add_system_to_stage(
                LOAD_LEVEL_STAGE,
                notify_level_loaded_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LoadingLevel>(),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                spawn_level_entities_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LoadedLevel>(),
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
            .add_system(rotate_goal_system.run_in_state(GameState::Game))
            .add_system(
                save_scene_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            );
    }
}

#[derive(Reflect, Resource, Deserialize, Serialize, TypeUuid, Debug)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct LevelTemplate {
    pub snakes: Vec<SnakeTemplate>,
    pub foods: Vec<IVec3>,
    pub walls: Vec<IVec3>,
    pub spikes: Vec<IVec3>,
}

#[derive(Resource)]
struct LoadingLevel(Handle<LevelTemplate>);

#[derive(Resource)]
pub struct LoadedLevel(pub Handle<LevelTemplate>);

#[derive(Default)]
pub struct LevelTemplateLoader;

impl AssetLoader for LevelTemplateLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let custom_asset = ron::de::from_bytes::<LevelTemplate>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["lvl"]
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

#[allow(clippy::too_many_arguments)]
fn spawn_level_entities_system(
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
    commands.spawn(DirectionalLightBundle {
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
        transform: Transform::from_translation(Vec3::new(0.5, 1.0, 0.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Spawn the wall blocks
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        base_color_texture: Some(assets.outline_texture.clone()),
        ..default()
    });

    for position in &level_template.walls {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                material: ground_material.clone(),
                transform: Transform::from_translation(position.as_vec3()),
                ..default()
            },
            LevelEntity,
        ));

        level_instance.mark_position_occupied(*position, LevelEntityType::Wall);
    }

    let mut mesh_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

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

fn save_scene_system(keyboard: Res<Input<KeyCode>>, level_instance: Res<LevelInstance>) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::S) {
        return;
    }

    let walls = level_instance
        .occupied_cells()
        .iter()
        .filter_map(|(position, cell_type)| match cell_type {
            LevelEntityType::Wall => Some(*position),
            _ => None,
        })
        .collect();

    let template = LevelTemplate {
        snakes: vec![vec![
            (IVec3::new(1, 1, 0), IVec3::X),
            (IVec3::new(0, 1, 0), IVec3::X),
        ]],
        foods: vec![IVec3::new(5, 1, 5)],
        spikes: vec![IVec3::new(10, 1, 10)],
        walls,
    };

    let ron_string = ron::ser::to_string_pretty(&template, PrettyConfig::default()).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            File::create("assets/level1.lvl")
                .and_then(|mut file| file.write(ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

pub fn spawn_spike(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    commands.spawn((
        mesh_builder.build_spike_mesh(*position),
        Spike(*position),
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

pub fn spawn_food(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    commands.spawn((
        mesh_builder.build_food_mesh(*position),
        Food(*position),
        LevelEntity,
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
    goal_query: Query<&Goal, With<Active>>,
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
