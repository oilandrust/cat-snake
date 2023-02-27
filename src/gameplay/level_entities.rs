use core::slice;

use bevy::{gltf::Gltf, prelude::*};

use crate::{
    level::level_instance::{EntityType, LevelGridEntity, LevelInstance},
    tools::picking::PickableBundle,
    GameAssets,
};

use super::{
    game_constants_plugin::{FOOD_COLOR, SPIKE_COLOR},
    snake_plugin::{Active, MaterialMeshBuilder, Snake, SnakeTemplate},
};

#[derive(Component)]
pub struct LevelEntity;

#[derive(Component, Clone, Copy)]
pub struct GridEntity {
    pub position: IVec3,
    pub entity_type: EntityType,
}

impl GridEntity {
    pub fn new(position: IVec3, entity_type: EntityType) -> Self {
        Self {
            position,
            entity_type,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct Wall;

#[derive(Component, Clone, Copy)]
pub struct Food;

#[derive(Component, Clone, Copy)]
pub struct Spike;

#[derive(Component, Clone, Copy)]
pub struct Goal;

#[derive(Component, Clone, Copy)]
pub struct Box;

#[derive(Component, Clone, Copy)]
pub struct Trigger;

pub trait Movable {
    fn positions(&self) -> &[IVec3];

    fn translate(&mut self, offset: IVec3);

    fn set_positions(&mut self, positions: &[IVec3]);

    fn entity_type(&self) -> EntityType;
}

impl Movable for GridEntity {
    fn positions(&self) -> &[IVec3] {
        slice::from_ref(&self.position)
    }

    fn translate(&mut self, offset: IVec3) {
        self.position += offset;
    }

    fn set_positions(&mut self, positions: &[IVec3]) {
        self.position = positions[0];
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Box
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
        Snake::new(snake_template, snake_index),
        SpatialBundle { ..default() },
        LevelEntity,
        Active,
        Name::new("Snake"),
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

pub fn spawn_spike(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) -> Entity {
    let entity = commands
        .spawn((
            mesh_builder.build_spike_mesh(*position),
            GridEntity::new(*position, EntityType::Spike),
            Spike,
            LevelEntity,
            Name::new("Spike"),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Spike));

    entity
}

pub fn spawn_wall(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
    assets: &GameAssets,
) -> Entity {
    let ground_material = mesh_builder.materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        base_color_texture: Some(assets.outline_texture.clone()),
        ..default()
    });

    let entity = commands
        .spawn((
            PbrBundle {
                mesh: assets.cube_mesh.clone(),
                material: ground_material,
                transform: Transform::from_translation(position.as_vec3()),
                ..default()
            },
            LevelEntity,
            GridEntity::new(*position, EntityType::Wall),
            Wall,
            Name::new("Wall"),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Wall));

    entity
}

pub fn spawn_food(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) -> Entity {
    let entity = commands
        .spawn((
            mesh_builder.build_food_mesh(*position),
            GridEntity::new(*position, EntityType::Food),
            Food,
            LevelEntity,
            Name::new("Food"),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Food));

    entity
}

pub fn spawn_box(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) -> Entity {
    let entity = commands
        .spawn((
            mesh_builder.build_box_mesh(*position),
            GridEntity::new(*position, EntityType::Box),
            Box,
            LevelEntity,
        ))
        .id();

    level_instance.mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Box));

    entity
}

pub fn spawn_goal(
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
    assets: &GameAssets,
    assets_gltf: &Assets<Gltf>,
) -> Entity {
    let entity = commands
        .spawn((
            SceneBundle {
                scene: assets_gltf.get(&assets.goal_inactive_mesh).unwrap().scenes[0].clone(),
                transform: Transform::from_translation(position.as_vec3()),
                ..default()
            },
            GridEntity::new(*position, EntityType::Goal),
            Goal,
            LevelEntity,
            Name::new("Goal"),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Goal));

    entity
}

pub fn spawn_trigger(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) -> Entity {
    let entity = commands
        .spawn((
            mesh_builder.build_trigger_mesh(*position),
            GridEntity::new(*position, EntityType::Trigger),
            Trigger,
            LevelEntity,
            PickableBundle::default(),
            Name::new("Trigger"),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Trigger));

    entity
}

impl<'a> MaterialMeshBuilder<'a> {
    pub fn build_box_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: self.materials.add(Color::BEIGE.into()),
            transform: Transform::from_translation(position.as_vec3()),
            ..default()
        }
    }

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

    pub fn build_trigger_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Box {
                min_x: -0.45,
                max_x: 0.45,
                min_y: -0.5,
                max_y: -0.3,
                min_z: -0.45,
                max_z: 0.45,
            })),
            material: self.materials.add(Color::GRAY.into()),
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
