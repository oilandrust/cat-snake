use bevy::prelude::*;

use crate::{
    level::level_instance::{EntityType, LevelGridEntity, LevelInstance},
    tools::picking::PickableBundle,
    GameAssets,
};

use super::{
    game_constants_pluggin::{FOOD_COLOR, SPIKE_COLOR},
    snake_pluggin::MaterialMeshBuilder,
};

#[derive(Component)]
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

#[derive(Component, Clone, Copy)]
pub struct Box;

pub trait Movable {
    fn positions(&self) -> Vec<IVec3>;

    fn translate(&mut self, offset: IVec3);
}

impl Movable for GridEntity {
    fn positions(&self) -> Vec<IVec3> {
        vec![self.0]
    }

    fn translate(&mut self, offset: IVec3) {
        self.0 += offset;
    }
}

pub fn spawn_spike(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    let entity = commands
        .spawn((
            mesh_builder.build_spike_mesh(*position),
            GridEntity(*position),
            Spike,
            LevelEntity,
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Spike));
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

    let entity = commands
        .spawn((
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
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Wall));
}

pub fn spawn_food(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    let entity = commands
        .spawn((
            mesh_builder.build_food_mesh(*position),
            GridEntity(*position),
            Food,
            LevelEntity,
            PickableBundle::default(),
        ))
        .id();

    level_instance
        .mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Food));
}

pub fn spawn_box(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
    level_instance: &mut LevelInstance,
) {
    let entity = commands
        .spawn((
            mesh_builder.build_box_mesh(*position),
            GridEntity(*position),
            Box,
            LevelEntity,
            PickableBundle::default(),
        ))
        .id();

    level_instance.mark_position_occupied(*position, LevelGridEntity::new(entity, EntityType::Box));
}

pub fn spawn_goal(
    mesh_builder: &mut MaterialMeshBuilder,
    commands: &mut Commands,
    position: &IVec3,
) {
    commands.spawn((
        mesh_builder.build_goal_mesh(*position),
        GridEntity(*position),
        Goal,
        LevelEntity,
        PickableBundle::default(),
    ));
}

impl<'a> MaterialMeshBuilder<'a> {
    pub fn build_box_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
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

    pub fn build_goal_mesh(&mut self, position: IVec3) -> PbrBundle {
        PbrBundle {
            mesh: self.meshes.add(Mesh::from(shape::Box {
                min_x: -0.4,
                max_x: 0.4,
                min_y: -0.5,
                max_y: -0.3,
                min_z: -0.4,
                max_z: 0.4,
            })),
            material: self.materials.add(Color::BLACK.into()),
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
