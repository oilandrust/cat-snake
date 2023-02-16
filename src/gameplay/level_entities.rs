use bevy::prelude::*;

use crate::{
    level::level_instance::{LevelEntityType, LevelInstance},
    tools::picking::PickableBundle,
    GameAssets,
};

use super::{
    game_constants_pluggin::{FOOD_COLOR, SPIKE_COLOR},
    snake_pluggin::MaterialMeshBuilder,
};

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
