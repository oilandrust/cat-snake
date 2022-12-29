use bevy::prelude::*;
use bevy_tweening::{Animator, EaseFunction, Lens, Tween};
use std::collections::VecDeque;

use crate::{
    game_constants_pluggin::{to_world, GRID_TO_WORLD_UNIT, SNAKE_SIZE},
    level::Level,
    level_pluggin::{Food, LevelEntity},
    movement_pluggin::{GravityFall, SnakeMovedEvent},
};

#[derive(Component)]
pub struct SnakePart(pub usize);

#[derive(Bundle)]
struct SnakePartBundle {
    spatial_bundle: SpatialBundle,
    part: SnakePart,
    level_entity: LevelEntity,
}

impl SnakePartBundle {
    fn new(position: IVec2, part_index: usize) -> Self {
        SnakePartBundle {
            spatial_bundle: SpatialBundle {
                transform: Transform {
                    translation: to_world(position).extend(0.0),
                    ..default()
                },
                ..default()
            },
            part: SnakePart(part_index),
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
}

pub struct SpawnSnakeEvent;

impl Snake {
    pub fn from_parts(parts: Vec<(IVec2, IVec2)>) -> Self {
        Self {
            parts: VecDeque::from(parts),
        }
    }

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
}

pub fn spawn_snake_system(
    mut commands: Commands,
    mut event_spawn_snake: EventReader<SpawnSnakeEvent>,
    level: Res<Level>,
) {
    if event_spawn_snake.iter().next().is_none() {
        return;
    }

    for (index, part) in level.initial_snake.iter().enumerate() {
        commands
            .spawn(SnakePartBundle::new(part.0, index))
            .with_children(|parent| {
                parent.spawn(SnakePartSpriteBundle::new(Vec2::ONE));
            });
    }

    commands
        .spawn(Snake::from_parts(level.initial_snake.clone()))
        .insert(LevelEntity);
}

pub fn respawn_snake_on_fall_system(
    mut spawn_snake_event: EventWriter<SpawnSnakeEvent>,
    mut commands: Commands,
    snake_query: Query<(Entity, &Snake), With<GravityFall>>,
    parts_query: Query<Entity, With<SnakePart>>,
) {
    let Ok((snake_entity, snake)) = snake_query.get_single() else {
        return;
    };

    if snake.head_position().y < -2 {
        spawn_snake_event.send(SpawnSnakeEvent);

        commands.entity(snake_entity).despawn();
        for snake_part_entity in &parts_query {
            commands.entity(snake_part_entity).despawn();
        }
    }
}

pub fn grow_snake_on_move_system(
    mut snake_moved_event: EventReader<SnakeMovedEvent>,
    mut commands: Commands,
    mut snake_query: Query<&mut Snake>,
    foods_query: Query<(Entity, &Food), With<Food>>,
) {
    if snake_moved_event.iter().next().is_none() {
        return;
    }

    let Ok(mut snake) = snake_query.get_single_mut() else {
        return;
    };

    for (food_entity, food) in &foods_query {
        if food.0 != snake.head_position() {
            continue;
        }

        commands.entity(food_entity).despawn();

        let tail_direction = snake.tail_direction();
        let new_part_position = snake.tail_position() - tail_direction;
        snake.parts.push_back((new_part_position, tail_direction));

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
            .spawn(SnakePartBundle::new(new_part_position, snake.len() - 1))
            .with_children(|parent| {
                parent
                    .spawn(SnakePartSpriteBundle::new(Vec2::ZERO))
                    .insert(Animator::new(grow_tween));
            });
    }
}
