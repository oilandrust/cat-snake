use std::{fs::File, io::Write};

use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_prototype_debug_lines::DebugLines;
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem},
    state::NextState,
};
use ron::ser::PrettyConfig;

use crate::{
    despawn_with,
    gameplay::{
        camera_plugin::{camera_pan_system, camera_zoom_scroll_system},
        level_pluggin::{
            clear_level_runtime_resources_system, spawn_level_entities_system, spawn_wall,
            CurrentLevelResourcePath, LevelEntity, LevelLoadedEvent, LevelTemplate, LoadingLevel,
        },
        snake_pluggin::{
            spawn_snake_system, update_snake_transforms_system, MaterialMeshBuilder, Snake,
            SpawnSnakeEvent,
        },
    },
    level::level_instance::{LevelEntityType, LevelInstance},
    utils::ray_from_screen_space,
    GameAssets, GameState,
};

use super::{
    dev_tools_pluggin::draw_cross,
    picking::{DefaultPickingPlugins, Selection},
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_system(start_editor_system.run_in_state(GameState::Game))
            .add_system(stop_editor_system.run_in_state(GameState::Editor))
            .add_enter_system(GameState::Editor, init_level_instance_system)
            .add_exit_system(GameState::Editor, despawn_with::<LevelEntity>)
            .add_exit_system(GameState::Editor, clear_level_runtime_resources_system)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>()
                    .with_system(spawn_level_entities_system)
                    .with_system(spawn_snake_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .with_system(add_wall_on_click_system)
                    .with_system(delete_selected_wall_system)
                    .with_system(save_level_system)
                    .with_system(update_snake_transforms_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>()
                    .with_system(camera_zoom_scroll_system)
                    .with_system(camera_pan_system)
                    .into(),
            );
    }
}

fn start_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut level_loaded_event: EventWriter<LevelLoadedEvent>,
    mut spawn_snake_event: EventWriter<SpawnSnakeEvent>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(NextState(GameState::Editor));
    level_loaded_event.send(LevelLoadedEvent);
    spawn_snake_event.send(SpawnSnakeEvent);
}

fn init_level_instance_system(mut commands: Commands) {
    commands.insert_resource(LevelInstance::new());
}

fn stop_editor_system(
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    current_level_asset_path: Res<CurrentLevelResourcePath>,
    asset_server: Res<AssetServer>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(LoadingLevel(asset_server.load(&current_level_asset_path.0)));
    commands.insert_resource(NextState(GameState::Game));
}

#[allow(clippy::too_many_arguments)]
fn add_wall_on_click_system(
    buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    windows: Res<Windows>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut level_instance: ResMut<LevelInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
) {
    if !keyboard.pressed(KeyCode::LControl) || !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = windows.get_primary().unwrap();
    let Some(mouse_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = camera.single();
    let ray = ray_from_screen_space(mouse_position, camera, camera_transform);

    let mut mesh_builder = MaterialMeshBuilder {
        meshes: meshes.as_mut(),
        materials: materials.as_mut(),
    };

    if let Some(position) = level_instance.find_first_free_cell_on_ray(ray) {
        spawn_wall(
            &mut mesh_builder,
            &mut commands,
            &position,
            &mut level_instance,
            assets.as_ref(),
        );
    }
}

fn delete_selected_wall_system(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    selection: Query<(Entity, &Selection)>,
) {
    if !keyboard.just_pressed(KeyCode::Back) {
        return;
    }

    for (entity, selection) in &selection {
        if !selection.selected() {
            continue;
        }

        commands.entity(entity).despawn();
    }
}

fn save_level_system(
    keyboard: Res<Input<KeyCode>>,
    level_instance: Res<LevelInstance>,
    snake_query: Query<&Snake>,
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::S) {
        return;
    }

    let walls = level_instance
        .occupied_cells()
        .iter()
        .filter_map(|(position, cell_type)| match cell_type {
            LevelEntityType::Wall => Some(*position),
            _ => None,
        })
        .collect();

    let foods = level_instance
        .occupied_cells()
        .iter()
        .filter_map(|(position, cell_type)| match cell_type {
            LevelEntityType::Food => Some(*position),
            _ => None,
        })
        .collect();

    let spikes = level_instance
        .occupied_cells()
        .iter()
        .filter_map(|(position, cell_type)| match cell_type {
            LevelEntityType::Spike => Some(*position),
            _ => None,
        })
        .collect();

    let snakes = snake_query
        .iter()
        .map(|snake| snake.parts().clone().into())
        .collect();

    let template = LevelTemplate {
        snakes,
        foods,
        spikes,
        walls,
    };

    let ron_string = ron::ser::to_string_pretty(&template, PrettyConfig::default()).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            File::create("assets/level1.lvl")
                .and_then(|mut file| file.write(ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}
