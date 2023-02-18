use crate::{
    gameplay::movement_pluggin::GravityFall,
    gameplay::snake_pluggin::Snake,
    gameplay::undo::{BeginFall, EndFall, MoveHistoryEvent, SnakeHistory},
    level::level_instance::{EntityType, LevelGridEntity, LevelInstance},
};
use bevy::prelude::*;

use super::level_entities::{GridEntity, Movable};

/// Provides commands that implement the undoable game mechanics.
/// Commands manage the state of the game data such as snakes, food, etc..
/// In addition they propagate the changes to the level instance that keep track of which object occupies which position.
/// Finaly, commands make sure that the changes are generate undoable instructions that can be executed by the undo system.
pub struct SnakeCommands<'a> {
    level_instance: &'a mut LevelInstance,
    history: &'a mut SnakeHistory,
}

impl<'a> SnakeCommands<'a> {
    pub fn new(level_instance: &'a mut LevelInstance, history: &'a mut SnakeHistory) -> Self {
        SnakeCommands {
            level_instance,
            history,
        }
    }

    pub fn player_move(
        &mut self,
        snake: &'a mut Snake,
        entity: Entity,
        direction: IVec3,
    ) -> PlayerMoveCommand {
        PlayerMoveCommand {
            level_instance: self.level_instance,
            history: self.history,
            snake,
            entity,
            pushed_entity: None,
            food: None,
            direction,
        }
    }

    pub fn exit_level(&mut self, snake: &'a Snake, entity: Entity, falling: Option<&GravityFall>) {
        let updates = if falling.is_none() {
            self.level_instance.clear_posisitons(&snake.positions())
        } else {
            vec![]
        };

        self.history.push_with_updates(
            MoveHistoryEvent::ExitLevel(entity),
            LevelGridEntity::new(entity, EntityType::Snake),
            updates,
        );
    }

    /// Execute a command when a skake start falling.
    pub fn start_falling(&mut self, snake: &'a dyn Movable, entity: LevelGridEntity) {
        let updates = self.level_instance.clear_posisitons(&snake.positions());

        self.history.push_with_updates(
            MoveHistoryEvent::BeginFall(BeginFall {
                positions: snake.positions(),
                end: None,
            }),
            entity,
            updates,
        );
    }

    pub fn stop_falling(&mut self, snake: &'a dyn Movable, entity: LevelGridEntity) {
        let updates = self
            .level_instance
            .mark_entity_positions(&snake.positions(), entity);

        // Stop fall can happen a long time after beggin fall, and other actions can be done in between.
        // We find the corresponding beggin fall and add the undo info to it so that both can be undone at the same time.
        let begin_fall = self
            .history
            .move_history
            .iter_mut()
            .rev()
            .find(|event| {
                event.level_entity.entity == entity.entity
                    && matches!(event.event, MoveHistoryEvent::BeginFall(_))
            })
            .unwrap();

        if let MoveHistoryEvent::BeginFall(begin) = &mut begin_fall.event {
            begin.end = Some(EndFall {
                walkable_updates: updates,
            })
        }
    }

    pub fn stop_falling_on_spikes(&mut self, entity: Entity) {
        // Stop fall can happen a long time after beggin fall, and other actions can be done in between.
        // We find the corresponding beggin fall and add the undo info to it so that both can be undone at the same time.
        let begin_fall = self
            .history
            .move_history
            .iter_mut()
            .rev()
            .find(|event| {
                event.level_entity.entity == entity
                    && matches!(event.event, MoveHistoryEvent::BeginFall(_))
            })
            .unwrap();

        if let MoveHistoryEvent::BeginFall(begin) = &mut begin_fall.event {
            begin.end = Some(EndFall {
                walkable_updates: vec![],
            })
        }
    }
}

pub struct PlayerMoveCommand<'a> {
    level_instance: &'a mut LevelInstance,
    history: &'a mut SnakeHistory,
    snake: &'a mut Snake,
    entity: Entity,
    pushed_entity: Option<(LevelGridEntity, &'a mut dyn Movable)>,
    food: Option<&'a GridEntity>,
    direction: IVec3,
}

impl<'a> PlayerMoveCommand<'a> {
    pub fn pushing_entity(
        mut self,
        movable: Option<(LevelGridEntity, &'a mut dyn Movable)>,
    ) -> Self {
        self.pushed_entity = movable;
        self
    }

    pub fn eating_food(mut self, food: Option<&'a GridEntity>) -> Self {
        self.food = food;
        self
    }

    pub fn execute(&mut self) {
        // Push the player action marker.
        self.history.push(
            MoveHistoryEvent::PlayerSnakeMove,
            LevelGridEntity::new(self.entity, EntityType::Snake),
        );

        // Move the other entity.
        if let Some((entity, movable)) = &mut self.pushed_entity {
            let walkable_updates =
                self.level_instance
                    .move_entity(*movable, *entity, self.direction);

            movable.translate(self.direction);

            self.history.push_with_updates(
                MoveHistoryEvent::PassiveEntityMove(self.direction),
                *entity,
                walkable_updates,
            );
        };

        // Consume food.
        if let Some(food) = &self.food {
            let walkable_updates = self.level_instance.eat_food(food.0);
            self.history.push_with_updates(
                MoveHistoryEvent::Eat(food.0),
                LevelGridEntity::new(self.entity, EntityType::Food),
                walkable_updates,
            );
        }

        // Then move the selected snake.
        let old_tail = self.snake.tail();
        let updates =
            self.level_instance
                .move_snake_forward(self.snake, self.entity, self.direction);

        self.snake.move_forward(self.direction);

        self.history.push_with_updates(
            MoveHistoryEvent::SnakeMoveForward(old_tail),
            LevelGridEntity::new(self.entity, EntityType::Snake),
            updates,
        );

        // Grow.
        if self.food.is_some() {
            let walkable_updates = self.level_instance.grow_snake(self.snake, self.entity);
            self.snake.grow();

            self.history.push_with_updates(
                MoveHistoryEvent::Grow,
                LevelGridEntity::new(self.entity, EntityType::Snake),
                walkable_updates,
            );
        }
    }
}
