use bevy::{prelude::*, render::primitives::Aabb, transform::TransformSystem};
use iyes_loopless::prelude::{ConditionHelpers, IntoConditionalSystem};
use std::collections::VecDeque;

use crate::{
    gameplay::commands::SnakeCommands,
    gameplay::game_constants_pluggin::SNAKE_COLORS,
    gameplay::level_entities::LevelEntity,
    gameplay::movement_pluggin::{GravityFall, MoveCommand, PushedAnim},
    gameplay::undo::{SnakeHistory, UndoEvent},
    level::level_instance::{EntityType, LevelGridEntity, LevelInstance},
    utils::{ray_from_screen_space, ray_intersects_aabb},
    GameState,
};

use crate::level::level_template::{LevelTemplate, LoadedLevel};

use super::level_entities::{GridEntity, Movable};

pub struct SnakePluggin;

impl Plugin for SnakePluggin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSnakeEvent>()
            .add_event::<DespawnSnakePartEvent>()
            .add_event::<DespawnSnakeEvent>()
            .add_event::<DespawnSnakePartsEvent>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                spawn_snake_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system(select_snake_mouse_system.run_in_state(GameState::Game))
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_snake_transforms_system
                    .run_in_state(GameState::Game)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_movable_transforms_system
                    .run_in_state(GameState::Game)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                despawn_snake_system
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                despawn_snake_part_system.run_in_state(GameState::Game),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                despawn_snake_parts_system.run_in_state(GameState::Game),
            );
    }
}

pub type SnakeElement = (IVec3, IVec3);
pub type SnakeTemplate = Vec<SnakeElement>;

#[derive(PartialEq, Eq)]
pub struct DespawnSnakePartEvent(pub SnakePart);

#[derive(PartialEq, Eq)]
pub struct DespawnSnakeEvent(pub i32);

#[derive(PartialEq, Eq)]
pub struct DespawnSnakePartsEvent(pub i32);

#[derive(Component)]
pub struct SelectedSnake;

#[derive(Component)]
pub struct Active;

#[derive(Component, PartialEq, Eq, Reflect, Clone)]
pub struct SnakePart {
    pub snake_index: i32,
    pub part_index: usize,
}

#[derive(Bundle)]
pub struct SnakePartBundle {
    pub part: SnakePart,
    pub level_entity: LevelEntity,
    pub shape: PbrBundle,
}

pub struct MaterialMeshBuilder<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
}

impl<'a> MaterialMeshBuilder<'a> {
    pub fn build_part(
        &mut self,
        position: IVec3,
        snake_index: i32,
        part_index: usize,
    ) -> SnakePartBundle {
        let color = SNAKE_COLORS[snake_index as usize][part_index % 2];
        let size = if part_index == 0 { 0.8 } else { 0.7 };

        SnakePartBundle {
            shape: PbrBundle {
                mesh: self.meshes.add(Mesh::from(shape::Cube { size })),
                material: self.materials.add(color.into()),
                global_transform: GlobalTransform::from_translation(position.as_vec3()),
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

#[derive(Component)]
pub struct PartClipper {
    pub clip_position: IVec3,
}

#[derive(Component, Debug)]
pub struct Snake {
    parts: VecDeque<SnakeElement>,
    index: i32,
}

pub struct SpawnSnakeEvent;

impl Snake {
    pub fn parts(&self) -> &VecDeque<SnakeElement> {
        &self.parts
    }

    pub fn get_part(&self, part_index: usize) -> &SnakeElement {
        &self.parts[part_index]
    }

    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn move_back(&mut self, part: &SnakeElement) {
        self.parts.push_back(*part);
        self.parts.pop_front();
    }

    pub fn move_forward(&mut self, direction: IVec3) {
        self.parts
            .push_front((self.head_position() + direction, direction));
        self.parts.pop_back();
    }

    pub fn head_position(&self) -> IVec3 {
        self.parts.front().unwrap().0
    }

    pub fn grow(&mut self) {
        let (tail_position, tail_direction) = self.tail();
        let new_part_position = tail_position - tail_direction;
        self.parts.push_back((new_part_position, tail_direction));
    }

    pub fn shrink(&mut self) {
        self.parts.pop_back();
    }

    pub fn tail(&self) -> SnakeElement {
        *self.parts.back().unwrap()
    }

    pub fn tail_position(&self) -> IVec3 {
        self.parts.back().unwrap().0
    }

    pub fn is_standing(&self) -> bool {
        (self.parts.front().unwrap().0.y - self.parts.back().unwrap().0.y)
            == (self.len() - 1) as i32
    }

    pub fn occupies_position(&self, position: IVec3) -> bool {
        self.parts.iter().any(|part| part.0 == position)
    }

    pub fn fall_one_unit(&mut self) {
        for (position, _) in self.parts.iter_mut() {
            *position += IVec3::NEG_Y;
        }
    }

    pub fn set_parts(&mut self, parts: Vec<SnakeElement>) {
        self.parts = parts.into();
    }
}

impl Movable for Snake {
    fn positions(&self) -> Vec<IVec3> {
        self.parts
            .iter()
            .copied()
            .map(|(position, _)| position)
            .collect()
    }

    fn translate(&mut self, offset: IVec3) {
        for (position, _) in self.parts.iter_mut() {
            *position += offset;
        }
    }
}

pub fn spawn_snake(
    part_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    level_instance: &mut LevelInstance,
    snake_template: &SnakeTemplate,
    snake_index: i32,
) -> Entity {
    let mut spawn_command = commands.spawn((
        Snake {
            parts: VecDeque::from(snake_template.clone()),
            index: snake_index,
        },
        SpatialBundle { ..default() },
        LevelEntity,
        Active,
    ));

    spawn_command.with_children(|parent| {
        for (index, part) in snake_template.iter().enumerate() {
            parent.spawn(part_builder.build_part(part.0, snake_index, index));
        }
    });

    for (position, _) in snake_template {
        level_instance.mark_position_occupied(
            *position,
            LevelGridEntity::new(spawn_command.id(), EntityType::Snake),
        );
    }

    spawn_command.id()
}

#[allow(clippy::type_complexity)]
pub fn update_snake_transforms_system(
    mut snake_query: Query<
        (
            &Snake,
            &mut Transform,
            &Children,
            Option<&MoveCommand>,
            Option<&PushedAnim>,
            Option<&GravityFall>,
        ),
        (With<Active>, Without<SnakePart>),
    >,
    mut part_query: Query<(&mut Transform, &SnakePart), With<SnakePart>>,
) {
    for (snake, mut transform, _, _, pushed_anim, fall) in &mut snake_query {
        let fall_offset = fall.map_or(Vec3::ZERO, |gravity_fall| gravity_fall.relative_z * Vec3::Y);

        let push_offset = pushed_anim.map_or(Vec3::ZERO, |command| {
            let initial_offset = -command.direction;
            initial_offset.lerp(Vec3::ZERO, command.lerp_time)
        });

        transform.translation = snake.head_position().as_vec3() + fall_offset + push_offset;
    }

    for (snake, _, children, move_command, _, _) in &mut snake_query {
        for child in children {
            let (mut part_transform, part) = part_query.get_mut(*child).unwrap();
            if part.part_index > snake.parts().len() - 1 {
                continue;
            }

            let element = snake.get_part(part.part_index);

            let move_offset = move_command.map_or(Vec3::ZERO, |command| {
                let initial_offset = -element.1.as_vec3();
                initial_offset.lerp(Vec3::ZERO, command.lerp_time)
            });

            part_transform.translation =
                (element.0 - snake.head_position()).as_vec3() + move_offset;
        }
    }
}

pub fn update_movable_transforms_system(
    mut moving_entitites: Query<(&GridEntity, &mut Transform, &PushedAnim)>,
) {
    for (grid_entity, mut transform, pushed_anim) in &mut moving_entitites {
        let initial_offset = -pushed_anim.direction;
        let push_offset = initial_offset.lerp(Vec3::ZERO, pushed_anim.lerp_time);

        transform.translation = grid_entity.0.as_vec3() + push_offset;
    }
}

pub fn set_snake_active(
    part_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    snake: &Snake,
    snake_entity: Entity,
) {
    commands
        .entity(snake_entity)
        .insert(Active)
        .with_children(|parent| {
            for (index, part) in snake.parts().iter().enumerate() {
                parent.spawn(part_builder.build_part(part.0, snake.index(), index));
            }
        });
}

pub fn spawn_snake_system(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut level_instance: ResMut<LevelInstance>,
    mut commands: Commands,
    mut event_spawn_snake: EventReader<SpawnSnakeEvent>,
    loaded_level: Res<LoadedLevel>,
    level_templates: ResMut<Assets<LevelTemplate>>,
) {
    if event_spawn_snake.iter().next().is_none() {
        return;
    }
    event_spawn_snake.clear();

    let level_template = level_templates
        .get(&loaded_level.0)
        .expect("Level should be loaded here!");

    let mut part_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    for (snake_index, snake_template) in level_template.snakes.iter().enumerate() {
        let entity = spawn_snake(
            &mut part_builder,
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

pub fn select_snake_mouse_system(
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform)>,
    selected_snake: Query<Entity, With<SelectedSnake>>,
    unselected_snakes: Query<(Entity, &Snake), Without<SelectedSnake>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = windows.get_primary().unwrap();

    let Some(mouse_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = camera.single();
    let ray = ray_from_screen_space(mouse_position, camera, camera_transform);

    let selected_snake_entity = selected_snake.single();

    let test_aabb = Aabb::from_min_max(0.5 * Vec3::NEG_ONE, 0.5 * Vec3::ONE);

    for (entity, snake) in unselected_snakes.iter() {
        let ray_hits_snake = snake.parts().iter().any(|(position, _)| {
            ray_intersects_aabb(ray, &test_aabb, &Mat4::from_translation(position.as_vec3()))
                .is_some()
        });

        if !ray_hits_snake {
            continue;
        }

        commands
            .entity(selected_snake_entity)
            .remove::<SelectedSnake>();

        commands.entity(entity).insert(SelectedSnake);
    }
}

pub fn respawn_snake_on_fall_system(
    mut snake_history: ResMut<SnakeHistory>,
    mut level: ResMut<LevelInstance>,
    mut trigger_undo_event: EventWriter<UndoEvent>,
    mut commands: Commands,
    mut snake_query: Query<(Entity, &Snake), With<GravityFall>>,
) {
    for (snake_entity, snake) in snake_query.iter_mut() {
        if snake.head_position().y >= -2 {
            return;
        }

        let mut snake_commands = SnakeCommands::new(&mut level, &mut snake_history);
        snake_commands.stop_falling(snake, snake_entity);

        commands.entity(snake_entity).remove::<GravityFall>();

        trigger_undo_event.send(UndoEvent);
    }
}

fn despawn_snake_system(
    mut despawn_snake_event: EventReader<DespawnSnakeEvent>,
    mut level_instance: ResMut<LevelInstance>,
    mut commands: Commands,
    snakes_query: Query<(Entity, &Snake)>,
    parts_query: Query<(Entity, &SnakePart)>,
) {
    for message in despawn_snake_event.iter() {
        // Despawn snake.
        for (entity, snake) in snakes_query.iter() {
            if snake.index != message.0 {
                continue;
            }

            commands.entity(entity).despawn_recursive();

            for (position, _) in &snake.parts {
                level_instance.set_empty(*position);
            }
        }

        // Despawn parts
        for (entity, part) in parts_query.iter() {
            if part.snake_index != message.0 {
                continue;
            }

            commands.entity(entity).despawn_recursive();
        }
    }
}

fn despawn_snake_part_system(
    mut despawn_snake_part_event: EventReader<DespawnSnakePartEvent>,
    mut commands: Commands,
    parts_query: Query<(Entity, &SnakePart)>,
) {
    for message in despawn_snake_part_event.iter() {
        for (entity, part) in parts_query.iter() {
            if *part != message.0 {
                continue;
            }

            commands.entity(entity).despawn_recursive();
        }
    }
}

fn despawn_snake_parts_system(
    mut despawn_snake_event: EventReader<DespawnSnakePartsEvent>,
    mut commands: Commands,
    parts_query: Query<(Entity, &SnakePart)>,
) {
    for message in despawn_snake_event.iter() {
        // Despawn parts
        for (entity, part) in parts_query.iter() {
            if part.snake_index != message.0 {
                continue;
            }

            commands.entity(entity).despawn_recursive();
        }
    }
}
