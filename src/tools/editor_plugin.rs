use std::{fs::File, io::Write};

use bevy::{prelude::*, tasks::IoTaskPool};
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionHelpers, ConditionSet, IntoConditionalSystem},
    state::NextState,
};
use ron::ser::PrettyConfig;

use crate::{
    despawn_with,
    gameplay::{
        camera_plugin::{camera_pan_system, camera_zoom_scroll_system},
        level_pluggin::{
            clear_level_runtime_resources_system, spawn_level_entities_system,
            CurrentLevelResourcePath, LevelEntity, LevelLoadedEvent, LevelTemplate, LoadingLevel,
        },
    },
    level::level_instance::{LevelEntityType, LevelInstance},
    GameState,
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(start_editor_system.run_in_state(GameState::Game))
            .add_system(stop_editor_system.run_in_state(GameState::Editor))
            .add_enter_system(GameState::Editor, init_level_instance_system)
            .add_exit_system(GameState::Editor, despawn_with::<LevelEntity>)
            .add_exit_system(GameState::Editor, clear_level_runtime_resources_system)
            .add_system(
                spawn_level_entities_system
                    .run_in_state(GameState::Editor)
                    .run_if_resource_exists::<LevelInstance>(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .with_system(add_wall_on_click_system)
                    .with_system(save_level_system)
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
) {
    if !keyboard.pressed(KeyCode::LWin) || !keyboard.just_pressed(KeyCode::E) {
        return;
    }

    commands.insert_resource(NextState(GameState::Editor));
    level_loaded_event.send(LevelLoadedEvent);
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

    let template: Handle<LevelTemplate> = asset_server.load(&current_level_asset_path.0);
    commands.insert_resource(LoadingLevel(template));
    commands.insert_resource(NextState(GameState::Game));
}

fn add_wall_on_click_system() {}

fn save_level_system(keyboard: Res<Input<KeyCode>>, level_instance: Res<LevelInstance>) {
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

    let template = LevelTemplate {
        snakes: vec![vec![
            (IVec3::new(1, 1, 0), IVec3::X),
            (IVec3::new(0, 1, 0), IVec3::X),
        ]],
        foods: vec![IVec3::new(5, 1, 5)],
        spikes: vec![IVec3::new(10, 1, 10)],
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
