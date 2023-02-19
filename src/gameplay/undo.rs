use bevy::prelude::*;

use crate::{
    gameplay::level_entities::*,
    gameplay::movement_plugin::GravityFall,
    gameplay::snake_plugin::{set_snake_active, DespawnSnakePartEvent, Snake, SnakePart},
    level::level_instance::{LevelGridEntity, LevelInstance},
};

use super::{
    level_entities::GridEntity,
    movement_plugin::MovableRegistry,
    snake_plugin::{MaterialMeshBuilder, SnakeElement},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LevelEntityUpdateEvent {
    ClearPosition(IVec3, LevelGridEntity),
    FillPosition(IVec3),
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BeginFall {
    // The initial position of the snake before falling.
    pub positions: Vec<IVec3>,

    // An even that is set when the fall ends.
    pub end: Option<EndFall>,
}

/// History event marking that a snake stops falling, with distance fallen.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EndFall {
    pub walkable_updates: Vec<LevelEntityUpdateEvent>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum MoveHistoryEvent {
    /// A history event that marks a player move action.
    PlayerSnakeMove,

    /// History event for the snake moving one tile in a direction, storing the old tails for undo.
    SnakeMoveForward(SnakeElement),

    /// History event for moving a snake with an offset fex: pushing.
    PassiveEntityMove(IVec3),

    /// History event marking that a snake starts falling.
    BeginFall(BeginFall),

    /// History event marking that a snake grew.
    Grow,

    /// History event when a snake eats a food and the food is despawned.
    Eat(IVec3),

    /// History event for a snake exiting the level through the goal.
    ExitLevel(Entity),
}

#[derive(Clone)]
pub struct SnakeHistoryEvent {
    pub event: MoveHistoryEvent,
    pub level_entity: LevelGridEntity,
    walkable_updates: Vec<LevelEntityUpdateEvent>,
}

pub struct UndoEvent;

/// A struct storing history events that can be undone.
#[derive(Resource, Default)]
pub struct SnakeHistory {
    pub move_history: Vec<SnakeHistoryEvent>,
}

impl SnakeHistory {
    pub fn push(&mut self, event: MoveHistoryEvent, level_entity: LevelGridEntity) {
        self.move_history.push(SnakeHistoryEvent {
            event,
            level_entity,
            walkable_updates: vec![],
        });
    }

    pub fn push_with_updates(
        &mut self,
        event: MoveHistoryEvent,
        level_entity: LevelGridEntity,
        walkable_updates: Vec<LevelEntityUpdateEvent>,
    ) {
        self.move_history.push(SnakeHistoryEvent {
            event,
            level_entity,
            walkable_updates,
        });
    }

    pub fn undo_last(
        &mut self,
        snakes: &mut Query<(Entity, &mut Snake)>,
        box_query: &mut Query<(Entity, &mut GridEntity), With<Box>>,
        level: &mut LevelInstance,
        commands: &mut Commands,
        part_builder: &mut MaterialMeshBuilder,
        despawn_snake_part_event: &mut EventWriter<DespawnSnakePartEvent>,
    ) {
        let mut movable_registry = MovableRegistry::new(snakes, box_query);

        // Undo the stack until we reach the last player action.
        while let Some(top) = self.move_history.pop() {
            if MoveHistoryEvent::PlayerSnakeMove == top.event {
                return;
            }

            match top.event {
                MoveHistoryEvent::PlayerSnakeMove => {
                    unreachable!("Should be handled as early return above.")
                }
                MoveHistoryEvent::SnakeMoveForward(old_tail) => {
                    let snake = movable_registry.get_mut_snake(&top.level_entity);
                    snake.move_back(&old_tail);
                }
                MoveHistoryEvent::PassiveEntityMove(offset) => {
                    let movable = movable_registry.get_mut(&top.level_entity);
                    movable.translate(-offset);
                }
                MoveHistoryEvent::BeginFall(begin) => {
                    let snake = movable_registry.get_mut(&top.level_entity);
                    snake.set_positions(&begin.positions);
                    if let Some(end) = begin.end {
                        level.undo_updates(&end.walkable_updates);
                    };
                }
                MoveHistoryEvent::Grow => {
                    let snake = movable_registry.get_mut_snake(&top.level_entity);
                    despawn_snake_part_event.send(DespawnSnakePartEvent(SnakePart {
                        snake_index: snake.index(),
                        part_index: snake.len() - 1,
                    }));

                    snake.shrink();
                }
                MoveHistoryEvent::Eat(position) => {
                    spawn_food(part_builder, commands, &position, level);
                }
                MoveHistoryEvent::ExitLevel(snake_entity) => {
                    let snake = movable_registry.get_mut_snake(&top.level_entity);
                    set_snake_active(part_builder, commands, snake, snake_entity);
                }
            }

            level.undo_updates(&top.walkable_updates);
        }
    }
}

pub fn keyboard_undo_system(
    keyboard: Res<Input<KeyCode>>,
    mut trigger_undo_event: EventWriter<UndoEvent>,
    falling_snakes: Query<(With<Snake>, With<GravityFall>)>,
) {
    if !keyboard.just_pressed(KeyCode::Back) {
        return;
    }

    if !falling_snakes.is_empty() {
        return;
    }

    trigger_undo_event.send(UndoEvent);
}

#[allow(clippy::too_many_arguments)]
pub fn undo_event_system(
    mut trigger_undo_event: EventReader<UndoEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut snake_history: ResMut<SnakeHistory>,
    mut level: ResMut<LevelInstance>,
    mut despawn_snake_part_event: EventWriter<DespawnSnakePartEvent>,
    mut commands: Commands,
    mut snake_query: Query<(Entity, &mut Snake)>,
    mut box_query: Query<(Entity, &mut GridEntity), With<Box>>,
) {
    if trigger_undo_event.iter().next().is_none() {
        return;
    }

    if snake_history.move_history.is_empty() {
        return;
    }

    let mut part_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    snake_history.undo_last(
        &mut snake_query,
        &mut box_query,
        &mut level,
        &mut commands,
        &mut part_builder,
        &mut despawn_snake_part_event,
    );
}
