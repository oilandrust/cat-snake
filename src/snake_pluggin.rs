use bevy::prelude::*;
use bevy_tweening::{Animator, EaseFunction, Lens, Tween};
use std::collections::VecDeque;

use crate::{
    game_constants_pluggin::{to_world, GRID_TO_WORLD_UNIT, SNAKE_SIZE},
    level_pluggin::{Food, LevelEntity, LevelInstance, Walkable},
    level_template::LevelTemplate,
    movement_pluggin::{
        update_sprite_positions_system, GravityFall, MoveHistoryEvent, SnakeHistory,
        SnakeMovedEvent, UndoEvent,
    },
};

pub struct SnakePluggin;

impl Plugin for SnakePluggin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnSnakePartEvent>()
            .add_system_to_stage(CoreStage::PreUpdate, spawn_snake_system)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                despawn_snake_part.after(update_sprite_positions_system),
            );
    }
}

#[derive(PartialEq, Eq)]
pub struct DespawnSnakePartEvent(pub SnakePart);

#[derive(Component)]
pub struct SelectedSnake;

#[derive(Component, PartialEq, Eq)]
pub struct SnakePart {
    pub snake_index: i32,
    pub part_index: usize,
}

#[derive(Bundle)]
struct SnakePartBundle {
    spatial_bundle: SpatialBundle,
    part: SnakePart,
    level_entity: LevelEntity,
}

impl SnakePartBundle {
    fn new(position: IVec2, snake_index: i32, part_index: usize) -> Self {
        SnakePartBundle {
            spatial_bundle: SpatialBundle {
                transform: Transform {
                    translation: to_world(position).extend(0.0),
                    ..default()
                },
                ..default()
            },
            part: SnakePart {
                snake_index,
                part_index,
            },
            level_entity: LevelEntity,
        }
    }
}

#[derive(Bundle)]
struct SnakePartSpriteBundle {
    sprite_bundle: SpriteBundle,
    level_entity: LevelEntity,
}

impl SnakePartSpriteBundle {
    fn new(scale: Vec2) -> Self {
        SnakePartSpriteBundle {
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: Color::GRAY,
                    custom_size: Some(SNAKE_SIZE),
                    ..default()
                },
                transform: Transform {
                    scale: scale.extend(1.0),
                    ..default()
                },
                ..default()
            },
            level_entity: LevelEntity,
        }
    }
}

struct GrowPartLens {
    scale_start: Vec2,
    scale_end: Vec2,
    grow_direction: Vec2,
}

impl Lens<Transform> for GrowPartLens {
    fn lerp(&mut self, target: &mut Transform, ratio: f32) {
        let value = self.scale_start + (self.scale_end - self.scale_start) * ratio;
        target.scale = value.extend(1.0);

        let mut offset = 0.5 * value * self.grow_direction - 0.5 * self.grow_direction;
        offset *= GRID_TO_WORLD_UNIT;
        let z = target.translation.z;
        target.translation = (offset).extend(z);
    }
}

#[derive(Component)]
pub struct Snake {
    pub parts: VecDeque<(IVec2, IVec2)>,
    pub index: i32,
}

pub struct SpawnSnakeEvent;

impl Snake {
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn head_position(&self) -> IVec2 {
        self.parts.front().unwrap().0
    }

    pub fn tail_position(&self) -> IVec2 {
        self.parts.back().unwrap().0
    }

    pub fn tail_direction(&self) -> IVec2 {
        self.parts.back().unwrap().1
    }

    pub fn is_standing(&self) -> bool {
        (self.parts.front().unwrap().0.y - self.parts.back().unwrap().0.y)
            == (self.len() - 1) as i32
    }

    pub fn occupies_position(&self, position: IVec2) -> bool {
        self.parts.iter().any(|part| part.0 == position)
    }

    pub fn fall_one_unit(&mut self) {
        for (position, _) in self.parts.iter_mut() {
            *position += IVec2::NEG_Y;
        }
    }

    pub fn move_up(&mut self, distance: i32) {
        for (position, _) in self.parts.iter_mut() {
            *position += IVec2::Y * distance;
        }
    }
}

pub fn spawn_snake_system(
    level: Res<LevelTemplate>,
    mut level_instance: ResMut<LevelInstance>,
    mut commands: Commands,
    mut event_spawn_snake: EventReader<SpawnSnakeEvent>,
) {
    if event_spawn_snake.iter().next().is_none() {
        return;
    }

    for (snake_index, snake_template) in level.initial_snakes.iter().enumerate() {
        for (index, part) in snake_template.iter().enumerate() {
            commands
                .spawn(SnakePartBundle::new(part.0, snake_index as i32, index))
                .with_children(|parent| {
                    parent.spawn(SnakePartSpriteBundle::new(Vec2::ONE));
                });
        }

        let mut spawn_command = commands.spawn(Snake {
            parts: VecDeque::from(snake_template.clone()),
            index: snake_index as i32,
        });

        spawn_command.insert(LevelEntity);

        if snake_index == 0 {
            spawn_command.insert(SelectedSnake);
        }

        for (position, _) in snake_template {
            level_instance.mark_position_walkable(*position, Walkable::Snake(snake_index as i32));
        }
    }
}

pub fn respawn_snake_on_fall_system(
    mut snake_history: ResMut<SnakeHistory>,
    mut trigger_undo_event: EventWriter<UndoEvent>,
    mut commands: Commands,
    mut snake_query: Query<(Entity, &Snake, &GravityFall)>,
) {
    for (snake_entity, snake, &gravity_fall) in snake_query.iter_mut() {
        if snake.head_position().y >= -2 {
            return;
        }

        snake_history.push(
            MoveHistoryEvent::Fall(gravity_fall.grid_distance),
            snake.index,
        );

        commands.entity(snake_entity).remove::<GravityFall>();

        trigger_undo_event.send(UndoEvent);
    }
}

pub fn grow_snake_on_move_system(
    mut snake_moved_event: EventReader<SnakeMovedEvent>,
    mut commands: Commands,
    mut level: ResMut<LevelInstance>,
    mut snake_history: ResMut<SnakeHistory>,
    mut snake_query: Query<&mut Snake>,
    foods_query: Query<(Entity, &Food), With<Food>>,
) {
    if snake_moved_event.iter().next().is_none() {
        return;
    }

    for mut snake in snake_query.iter_mut() {
        for (food_entity, food) in &foods_query {
            if food.0 != snake.head_position() {
                continue;
            }

            commands.entity(food_entity).despawn();

            level.set_empty(food.0);

            let tail_direction = snake.tail_direction();
            let new_part_position = snake.tail_position() - tail_direction;
            snake.parts.push_back((new_part_position, tail_direction));

            snake_history.push(MoveHistoryEvent::Eat(food.0), snake.index);

            let grow_tween = Tween::new(
                EaseFunction::QuadraticInOut,
                std::time::Duration::from_secs_f32(0.2),
                GrowPartLens {
                    scale_start: Vec2::ONE - tail_direction.as_vec2().abs(),
                    scale_end: Vec2::ONE,
                    grow_direction: -tail_direction.as_vec2(),
                },
            );

            commands
                .spawn(SnakePartBundle::new(
                    new_part_position,
                    snake.index,
                    snake.len() - 1,
                ))
                .with_children(|parent| {
                    parent
                        .spawn(SnakePartSpriteBundle::new(Vec2::ZERO))
                        .insert(Animator::new(grow_tween));
                });
        }
    }
}

fn despawn_snake_part(
    mut despawn_snake_part_event: EventReader<DespawnSnakePartEvent>,
    mut commands: Commands,
    mut snake_query: Query<&mut Snake>,
    parts_query: Query<(Entity, &SnakePart)>,
) {
    for message in despawn_snake_part_event.iter() {
        for (entity, part) in parts_query.iter() {
            if *part != message.0 {
                continue;
            }

            commands.entity(entity).despawn_recursive();
            snake_query
                .iter_mut()
                .find(|snake| snake.index == part.snake_index)
                .expect("Trying to despawn a part for a snake that is not found in query.")
                .parts
                .pop_back();
        }
    }
}