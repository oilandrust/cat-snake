use bevy::{ecs::query::ReadOnlyWorldQuery, prelude::*, utils::HashMap};
use bevy_kira_audio::{Audio, AudioControl};
use bevy_tweening::{
    component_animator_system, AnimationSystem, Animator, EaseFunction, Lens, Tween,
};
use iyes_loopless::prelude::{ConditionSet, IntoConditionalSystem};
use rand::prelude::*;

use crate::{
    gameplay::commands::SnakeCommands,
    gameplay::game_constants_plugin::*,
    gameplay::snake_plugin::{respawn_snake_on_fall_system, Active, SelectedSnake, Snake},
    gameplay::undo::{keyboard_undo_system, undo_event_system, SnakeHistory, UndoEvent},
    level::level_instance::{EntityType, LevelGridEntity, LevelInstance},
    GameAssets, GameState,
};

use super::{
    level_entities::*,
    snake_plugin::{
        DespawnSnakePartEvent, MaterialMeshBuilder, PartClipper, SnakeElement, SnakePart,
    },
};

const MOVE_UP_KEYS: [KeyCode; 2] = [KeyCode::W, KeyCode::Up];
const MOVE_LEFT_KEYS: [KeyCode; 2] = [KeyCode::A, KeyCode::Left];
const MOVE_DOWN_KEYS: [KeyCode; 2] = [KeyCode::S, KeyCode::Down];
const MOVE_RIGHT_KEYS: [KeyCode; 2] = [KeyCode::D, KeyCode::Right];
const RISE_KEYS: [KeyCode; 2] = [KeyCode::E, KeyCode::Space];
const DIVE_KEYS: [KeyCode; 2] = [KeyCode::Q, KeyCode::LControl];

#[derive(Component, Default)]
pub struct MoveCommand {
    velocity: f32,
    pub lerp_time: f32,
}

#[derive(Component, Default)]
pub struct PushedAnim {
    pub direction: Vec3,
    velocity: f32,
    pub lerp_time: f32,
}

#[derive(Component, Copy, Clone)]
pub struct GravityFall {
    velocity: f32,
    pub relative_z: f32,
    pub grid_distance: i32,
}

#[derive(Component, Clone)]
pub struct LevelExitAnim {
    pub distance_to_move: i32,
    pub initial_snake_position: Vec<SnakeElement>,
}

#[derive(Component)]
pub struct PartGrowAnim {
    pub grow_factor: f32,
}

struct GrowPartLens;

impl Lens<PartGrowAnim> for GrowPartLens {
    fn lerp(&mut self, target: &mut PartGrowAnim, ratio: f32) {
        target.grow_factor = ratio;
    }
}

pub struct MovementPlugin;

pub struct MoveCommandEvent(pub IVec3);

pub struct SnakeMovedEvent;

pub struct SnakeReachGoalEvent(pub Entity);

pub struct SnakeExitedLevelEvent;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel, StageLabel)]
pub enum MovementStages {
    KeyboardInput,
    Undo,
    SnakeMovement,
    SnakeGrow,
    SnakeFall,
    SmoothMovement,
}

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SnakeMovedEvent>()
            .add_event::<MoveCommandEvent>()
            .add_event::<SnakeReachGoalEvent>()
            .add_event::<SnakeExitedLevelEvent>()
            .add_event::<crate::gameplay::undo::UndoEvent>()
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::KeyboardInput)
                    .with_system(keyboard_undo_system)
                    .with_system(keyboard_move_command_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::Undo)
                    .after(MovementStages::KeyboardInput)
                    .with_system(undo_event_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::SnakeMovement)
                    .after(MovementStages::Undo)
                    .with_system(snake_movement_control_system)
                    .into(),
            )
            .add_system(
                grow_snake_on_move_system
                    .run_in_state(GameState::Game)
                    .label(MovementStages::SnakeGrow)
                    .after(MovementStages::SnakeMovement),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::SnakeFall)
                    .after(MovementStages::SnakeGrow)
                    .with_system(gravity_system::<Snake, (With<Active>, Without<LevelExitAnim>)>)
                    .with_system(gravity_system::<GridEntity, With<Box>>)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .label(MovementStages::SmoothMovement)
                    .after(MovementStages::SnakeFall)
                    .with_system(snake_smooth_movement_system)
                    .with_system(snake_push_anim_system)
                    .with_system(snake_exit_level_anim_system)
                    .with_system(respawn_snake_on_fall_system)
                    .with_system(activate_trigger_on_move_system)
                    .into(),
            )
            .add_system(
                component_animator_system::<PartGrowAnim>
                    .run_in_state(GameState::Game)
                    .label(AnimationSystem::AnimationUpdate),
            );
    }
}

fn min_distance_to_ground(
    level: &LevelInstance,
    entity_positions: &[IVec3],
    snake_entity: Entity,
) -> i32 {
    entity_positions
        .iter()
        .map(|position| level.get_distance_to_ground(*position, snake_entity))
        .min()
        .unwrap()
}

pub fn keyboard_move_command_system(
    keyboard: Res<Input<KeyCode>>,
    mut move_command_event: EventWriter<MoveCommandEvent>,
) {
    let new_direction = if keyboard.any_just_pressed(MOVE_UP_KEYS) {
        Some(IVec3::NEG_Z)
    } else if keyboard.any_just_pressed(MOVE_LEFT_KEYS) {
        Some(IVec3::NEG_X)
    } else if keyboard.any_just_pressed(MOVE_DOWN_KEYS) {
        Some(IVec3::Z)
    } else if keyboard.any_just_pressed(MOVE_RIGHT_KEYS) {
        Some(IVec3::X)
    } else if keyboard.any_just_pressed(RISE_KEYS) {
        Some(IVec3::Y)
    } else if keyboard.any_just_pressed(DIVE_KEYS) {
        Some(IVec3::NEG_Y)
    } else {
        None
    };

    let Some(direction) = new_direction else {
        return;
    };

    move_command_event.send(MoveCommandEvent(direction));
}

type WithMovementControlSystemFilter = (
    With<SelectedSnake>,
    With<Active>,
    Without<MoveCommand>,
    Without<GravityFall>,
);

fn snake_can_move_forward(
    level_instance: &LevelInstance,
    snake: &Snake,
    other_entity: &Option<(Entity, &dyn Movable)>,
    direction: IVec3,
) -> bool {
    let new_position = snake.head_position() + direction;

    if let Some(other_entity) = &other_entity {
        return level_instance.can_push_entity(
            other_entity.0,
            other_entity.1.positions(),
            direction,
        );
    };

    if snake.occupies_position(new_position) || !level_instance.can_walk_or_eat(new_position) {
        return false;
    }

    true
}

pub struct MovableRegistry<'a> {
    snake_registry: HashMap<Entity, Mut<'a, Snake>>,
    box_registry: HashMap<Entity, Mut<'a, GridEntity>>,
}

impl<'a> MovableRegistry<'a> {
    pub fn new<
        SnakeFilter: bevy::ecs::query::ReadOnlyWorldQuery,
        BoxFilter: bevy::ecs::query::ReadOnlyWorldQuery,
    >(
        snake_query: &'a mut Query<(Entity, &mut Snake), SnakeFilter>,
        box_query: &'a mut Query<(Entity, &mut GridEntity), BoxFilter>,
    ) -> Self {
        let mut snake_registry: HashMap<Entity, Mut<Snake>> = HashMap::new();
        for (entity, snake) in &mut *snake_query {
            snake_registry.insert(entity, snake);
        }

        let mut box_registry: HashMap<Entity, Mut<GridEntity>> = HashMap::new();
        for (entity, movable) in &mut *box_query {
            box_registry.insert(entity, movable);
        }

        Self {
            snake_registry,
            box_registry,
        }
    }

    pub fn get(&mut self, entity: &LevelGridEntity) -> &dyn Movable {
        match entity.entity_type {
            EntityType::Box => self.box_registry.get(&entity.entity).expect("msg").as_ref(),
            EntityType::Snake => self
                .snake_registry
                .get(&entity.entity)
                .expect("msg")
                .as_ref(),
            _ => panic!("Should not happen"),
        }
    }

    pub fn get_mut_snake(&mut self, entity: &LevelGridEntity) -> &mut Snake {
        self.snake_registry
            .get_mut(&entity.entity)
            .expect("msg")
            .as_mut()
    }

    pub fn get_mut(&mut self, entity: &LevelGridEntity) -> &mut dyn Movable {
        match entity.entity_type {
            EntityType::Box => {
                let movable_ref: &mut dyn Movable = self
                    .box_registry
                    .get_mut(&entity.entity)
                    .expect("msg")
                    .as_mut();
                movable_ref
            }
            EntityType::Snake => {
                let movable_ref: &mut dyn Movable = self
                    .snake_registry
                    .get_mut(&entity.entity)
                    .expect("msg")
                    .as_mut();
                movable_ref
            }
            _ => panic!("Should not happen"),
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn snake_movement_control_system(
    assets: Res<GameAssets>,
    audio: Res<Audio>,
    mut level_instance: ResMut<LevelInstance>,
    constants: Res<GameConstants>,
    mut snake_history: ResMut<SnakeHistory>,
    mut move_command_event: EventReader<MoveCommandEvent>,
    mut _snake_reach_goal_event: EventWriter<SnakeReachGoalEvent>,
    mut commands: Commands,
    mut snake_moved_event: EventWriter<SnakeMovedEvent>,
    mut selected_snake_query: Query<(Entity, &mut Snake), WithMovementControlSystemFilter>,
    mut other_snakes_query: Query<(Entity, &mut Snake), Without<SelectedSnake>>,
    mut boxes_query: Query<(Entity, &mut GridEntity), (With<Box>, Without<Food>)>,
    foods_query: Query<&GridEntity, (With<Food>, Without<Box>)>,
    goal_query: Query<&GridEntity, (With<Goal>, With<Active>, Without<Box>, Without<Food>)>,
) {
    let Ok((snake_entity, mut snake)) = selected_snake_query.get_single_mut() else {
        return;
    };

    let Some(MoveCommandEvent(direction)) = move_command_event.iter().next() else {
        return;
    };

    if *direction == -snake.head_direction() {
        return;
    }

    let mut movable_registry = MovableRegistry::new(&mut other_snakes_query, &mut boxes_query);

    // We try to move with the input direction, if not possible try to go up.
    let directions = vec![*direction, IVec3::Y];

    let move_forward_or_up = 'choose_direction: {
        for direction in directions {
            let new_position = snake.head_position() + direction;

            // Check that we have enough parts to go up.
            let is_goal = if let Ok(goal) = goal_query.get_single() {
                goal.position == new_position
            } else {
                false
            };

            if direction == IVec3::Y
                && snake.is_standing()
                && !level_instance.is_food(new_position)
                && !is_goal
            {
                commands.entity(snake_entity).insert(GravityFall {
                    velocity: constants.jump_velocity,
                    relative_z: 0.0,
                    grid_distance: 0,
                });
                break 'choose_direction None;
            }

            if level_instance.is_entity(new_position, snake_entity) {
                continue;
            }

            // Find if there is a movable entity in the way.
            let movable_entity = level_instance.is_movable(new_position);
            let movable =
                movable_entity.map(|entity| (entity.entity, movable_registry.get(&entity)));

            // Check if we can move forward.
            if snake_can_move_forward(&level_instance, &snake, &movable, direction) {
                break 'choose_direction Some((direction, new_position, movable_entity));
            }
        }
        None
    };

    let Some((direction, new_position, movable_entity)) = move_forward_or_up else {
        return;
    };

    // Any food?
    let food = foods_query
        .iter()
        .find(|food| food.position == new_position);

    // Finaly move the snake forward and commit the state.
    let mut snake_commands = SnakeCommands::new(&mut level_instance, &mut snake_history);

    let movable = movable_entity.map(|entity| (entity, movable_registry.get_mut(&entity)));

    snake_commands
        .player_move(snake.as_mut(), snake_entity, direction)
        .pushing_entity(movable)
        .eating_food(food)
        .execute();

    // if let Ok(goal) = goal_query.get_single() {
    //     if snake.head_position() == goal.0 {
    //         snake_reach_goal_event.send(SnakeReachGoalEvent(snake_entity));
    //     }
    // }

    snake_moved_event.send(SnakeMovedEvent);

    // Smooth move animation starts.
    commands.entity(snake_entity).insert(MoveCommand {
        velocity: constants.move_velocity,
        lerp_time: 0.0,
    });

    if let Some(other_snake_entity) = movable_entity {
        commands
            .entity(other_snake_entity.entity)
            .insert(PushedAnim {
                direction: direction.as_vec3(),
                velocity: constants.move_velocity,
                lerp_time: 0.0,
            });
    }

    audio
        .play(assets.move_effect.clone())
        .with_playback_rate(1.0 + rand::thread_rng().gen_range(-0.05..0.1))
        .with_volume(2.0);
}

pub fn grow_snake_on_move_system(
    mut snake_moved_event: EventReader<SnakeMovedEvent>,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    mut materials: ResMut<bevy::asset::Assets<StandardMaterial>>,
    mut commands: Commands,
    snake_query: Query<(Entity, &Snake), With<SelectedSnake>>,
    foods_query: Query<(Entity, &GridEntity), With<Food>>,
) {
    if snake_moved_event.iter().next().is_none() {
        return;
    }

    let Ok((snake_entity, snake)) = snake_query.get_single() else {
        return;
    };

    for (food_entity, food) in &foods_query {
        if food.position != snake.head_position() {
            continue;
        }

        commands.entity(food_entity).despawn();

        let grow_tween = Tween::new(
            EaseFunction::QuadraticInOut,
            std::time::Duration::from_secs_f32(0.2),
            GrowPartLens,
        );

        let mut part_builder = MaterialMeshBuilder {
            meshes: meshes.as_mut(),
            materials: materials.as_mut(),
        };

        commands.entity(snake_entity).with_children(|parent| {
            parent
                .spawn(part_builder.build_part(
                    snake.tail_position(),
                    snake.index(),
                    snake.len() - 1,
                ))
                .insert((Animator::new(grow_tween), PartGrowAnim { grow_factor: 0.0 }));
        });
    }
}

pub fn activate_trigger_on_move_system(
    level_instance: Res<LevelInstance>,
    snake_moved_event: EventReader<SnakeMovedEvent>,
    undo_event: EventReader<UndoEvent>,
    mut commands: Commands,
    mut triggers: Query<(Entity, &mut Transform, &GridEntity, Option<&Active>), With<Trigger>>,
) {
    if snake_moved_event.is_empty() && undo_event.is_empty() {
        return;
    }

    for (trigger_entity, mut transform, trigger, active) in &mut triggers {
        let has_load = level_instance.is_movable(trigger.position).is_some();

        if has_load && active.is_none() {
            commands.entity(trigger_entity).insert(Active);
            transform.translation -= 0.18 * Vec3::Y;
        } else if !has_load && active.is_some() {
            transform.translation += 0.18 * Vec3::Y;
            commands.entity(trigger_entity).remove::<Active>();
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn gravity_system<MovableType, Filter>(
    time: Res<Time>,
    constants: Res<GameConstants>,
    mut level: ResMut<LevelInstance>,
    mut snake_history: ResMut<SnakeHistory>,
    mut _trigger_undo_event: EventWriter<UndoEvent>,
    mut snake_reach_goal_event: EventReader<SnakeReachGoalEvent>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut MovableType, Option<&mut GravityFall>), Filter>,
) where
    MovableType: Component + Movable,
    Filter: ReadOnlyWorldQuery,
{
    for (movable_entity, mut movable, gravity_fall) in &mut query {
        if snake_reach_goal_event
            .iter()
            .any(|event| event.0 == movable_entity)
        {
            continue;
        }

        match gravity_fall {
            None => {
                // Check if snake is on the ground and spawn gravity fall if not.
                if min_distance_to_ground(&level, movable.positions(), movable_entity) <= 1 {
                    continue;
                }

                let mut snake_commands = SnakeCommands::new(&mut level, &mut snake_history);
                snake_commands.start_falling(
                    movable.as_ref(),
                    LevelGridEntity::new(movable_entity, movable.entity_type()),
                );

                movable.translate(IVec3::NEG_Y);

                commands.entity(movable_entity).insert(GravityFall {
                    velocity: 0.0,
                    relative_z: 1.0,
                    grid_distance: 1,
                });
            }
            Some(mut gravity_fall) => {
                gravity_fall.velocity -= constants.gravity * time.delta_seconds();
                gravity_fall.relative_z += gravity_fall.velocity * time.delta_seconds();

                // While relative y is positive, we haven't moved fully into the cell.
                if gravity_fall.relative_z >= 0.0 {
                    continue;
                }

                // Check if we fell on spikes, if, so trigger undo.
                // for position in snake.positions() {
                //     if !level.is_spike(position) {
                //         continue;
                //     }

                //     let mut snake_commands = SnakeCommands::new(&mut level, &mut snake_history);
                //     snake_commands.stop_falling_on_spikes(snake_entity);

                //     commands.entity(snake_entity).remove::<GravityFall>();

                //     trigger_undo_event.send(UndoEvent);
                //     return;
                // }

                // keep falling..
                if min_distance_to_ground(&level, movable.positions(), movable_entity) > 1 {
                    gravity_fall.relative_z = 1.0;
                    gravity_fall.grid_distance += 1;

                    movable.translate(IVec3::NEG_Y);
                } else {
                    // ..or stop falling animation.
                    commands.entity(movable_entity).remove::<GravityFall>();

                    // Nothing to do if we fell less than an unit, meaning we stayed at the same place.
                    if gravity_fall.grid_distance == 0 {
                        return;
                    }

                    let mut snake_commands = SnakeCommands::new(&mut level, &mut snake_history);
                    snake_commands.stop_falling(
                        movable.as_ref(),
                        LevelGridEntity::new(movable_entity, movable.entity_type()),
                    );
                }
            }
        }
    }
}

fn snake_smooth_movement_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut MoveCommand)>,
) {
    for (entity, mut move_command) in query.iter_mut() {
        move_command.lerp_time += move_command.velocity * time.delta_seconds();
        if move_command.lerp_time > 1.0 {
            commands.entity(entity).remove::<MoveCommand>();
        }
    }
}

pub fn snake_push_anim_system(
    time: Res<Time>,
    mut commands: Commands,
    mut push_anim_query: Query<(Entity, &mut PushedAnim)>,
) {
    for (entity, mut move_command) in push_anim_query.iter_mut() {
        move_command.lerp_time += move_command.velocity * time.delta_seconds();
        if move_command.lerp_time > 1.0 {
            commands.entity(entity).remove::<PushedAnim>();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn snake_exit_level_anim_system(
    constants: Res<GameConstants>,
    mut commands: Commands,
    mut event_despawn_snake_parts: EventWriter<DespawnSnakePartEvent>,
    mut event_snake_exited_level: EventWriter<SnakeExitedLevelEvent>,
    mut anim_query: Query<(
        Entity,
        &mut Snake,
        &mut LevelExitAnim,
        Option<&MoveCommand>,
        &Children,
    )>,
    mut snake_part_query: Query<(Entity, &SnakePart, Option<&mut PartClipper>)>,
    goal_query: Query<&GridEntity, (With<Goal>, With<Active>)>,
) {
    let Ok(goal) = goal_query.get_single() else {
        return;
    };

    for (entity, mut snake, mut level_exit, move_command, children) in anim_query.iter_mut() {
        for &child in children {
            let Ok((entity, part, modifier)) = snake_part_query.get_mut(child) else {
                continue;
            };

            if modifier.is_some() {
                if (snake.parts()[part.part_index].0 - goal.position)
                    .abs()
                    .max_element()
                    > 1
                {
                    event_despawn_snake_parts.send(DespawnSnakePartEvent(part.clone()));
                }
            } else if snake.parts()[part.part_index].0 == goal.position {
                commands.entity(entity).insert(PartClipper {
                    clip_position: goal.position,
                });
            }
        }

        if move_command.is_some() {
            continue;
        }

        level_exit.distance_to_move -= 1;

        if level_exit.distance_to_move < 0 {
            commands
                .entity(entity)
                .remove::<LevelExitAnim>()
                .remove::<Active>();

            event_snake_exited_level.send(SnakeExitedLevelEvent);

            snake.set_parts(level_exit.initial_snake_position.clone());
        } else {
            commands.entity(entity).insert(MoveCommand {
                velocity: 1.5 * constants.move_velocity,
                lerp_time: 0.0,
            });
            let direction = IVec3::NEG_Y;
            snake.move_forward(direction);
        }
    }
}
