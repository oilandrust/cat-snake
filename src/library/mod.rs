use bevy::{gltf::Gltf, prelude::*};
use bevy_kira_audio::AudioSource;

#[derive(Resource, Reflect)]
pub struct GameAssets {
    pub move_effect: Handle<AudioSource>,
    pub outline_texture: Handle<Image>,
    pub cube_mesh: Handle<Mesh>,
    pub default_material: Handle<StandardMaterial>,
    pub goal_light_cone_mesh: Handle<Mesh>,
    pub goal_light_cone_material: Handle<StandardMaterial>,
    pub goal_active_mesh: Handle<Gltf>,
    pub goal_inactive_mesh: Handle<Gltf>,
    pub kitchen_model: Handle<Gltf>,
}

pub fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GameAssets {
        move_effect: asset_server.load("move_effect.mp3"),
        outline_texture: asset_server.load("outline.png"),
        cube_mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        goal_light_cone_mesh: asset_server.load("goal.gltf#Mesh0/Primitive0"),
        goal_inactive_mesh: asset_server.load("goal_inactive.gltf"),
        goal_active_mesh: asset_server.load("goal_active.gltf"),
        kitchen_model: asset_server.load("kitchen.gltf"),
        goal_light_cone_material: materials.add(StandardMaterial {
            base_color: Color::rgba_u8(255, 255, 153, 150),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        default_material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.8, 0.7, 0.6),
            ..default()
        }),
    });
}
