use args::Args;
use bevy::prelude::*;
use bevy_kira_audio::AudioPlugin;
use bevy_tweening::TweeningPlugin;
use gameplay::camera_plugin::CameraPlugin;
use gameplay::game_constants_plugin::*;
use gameplay::level_entities::LevelEntity;
use gameplay::level_plugin::{
    ClearLevelEvent, CurrentLevelMetadata, LevelPlugin, StartLevelEventWithIndex,
    StartLevelEventWithLevelAssetPath,
};
use gameplay::movement_plugin::MovementPlugin;
use gameplay::snake_plugin::SnakePlugin;
use iyes_loopless::prelude::IntoConditionalSystem;
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionSet},
    state::NextState,
};
use library::load_assets;
use menus::main_menu::MainMenuPlugin;
use menus::select_level_menu::{NextLevel, SelectLevelMenuPlugin};
use menus::MenuPlugin;
use tools::dev_tools_plugin::DevToolsPlugin;
use tools::editor_plugin::{EditorPlugin, ResumeFromEditor};

pub mod args;
pub mod gameplay;
pub mod level;
pub mod library;
pub mod menus;
pub mod tools;
pub mod utils;

// Don't touch this piece, needed for Web
#[cfg(target_arch = "wasm32")]
mod web_main;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    MainMenu,
    SelectLevelMenu,
    Game,
    Editor,
}

pub struct GamePlugin {
    args: Args,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_exit_system(GameState::Game, despawn_with_system::<LevelEntity>)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .with_system(back_to_menu_on_escape_system)
                    .into(),
            )
            .add_plugin(LevelPlugin)
            .add_plugin(SnakePlugin)
            .add_plugin(MovementPlugin)
            .add_plugin(GameConstantsPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(DevToolsPlugin)
            .add_plugin(TweeningPlugin)
            .add_plugin(EditorPlugin)
            .insert_resource(self.args.clone())
            .insert_resource(NextLevel(self.args.level.unwrap_or(0)));

        //if let Some(args::Commands::Test { test_case: _ }) = self.args.command {
        //app.add_plugin(AutomatedTestPlugin);
        //}

        app.add_enter_system(
            GameState::Game,
            enter_game_system.run_unless_resource_exists::<ResumeFromEditor>(),
        )
        .add_enter_system(
            GameState::Game,
            return_from_editor_system.run_if_resource_exists::<ResumeFromEditor>(),
        );
    }
}

fn enter_game_system(
    mut commands: Commands,
    args: Res<Args>,
    next_level: Res<NextLevel>,
    // mut start_test_case_event: EventWriter<StartTestCaseEventWithIndex>,
    mut start_test_level_event: EventWriter<StartLevelEventWithLevelAssetPath>,
    mut start_level_event: EventWriter<StartLevelEventWithIndex>,
) {
    match args.command {
        Some(args::Commands::Test { test_case: _ }) => {
            // let start_test_case = test_case.unwrap_or(0);
            // start_test_case_event.send(StartTestCaseEventWithIndex(start_test_case));
        }
        _ => {
            if let Some(test_level) = &args.test_level {
                let level_asset_path = format!("levels/{}", test_level);
                commands.insert_resource(CurrentLevelMetadata {
                    id: None,
                    asset_path: level_asset_path.clone(),
                });

                start_test_level_event.send(StartLevelEventWithLevelAssetPath(level_asset_path));
                return;
            }
        }
    };

    start_level_event.send(StartLevelEventWithIndex(next_level.0));
}

fn return_from_editor_system(mut commands: Commands) {
    commands.remove_resource::<ResumeFromEditor>();
}

fn back_to_menu_on_escape_system(
    mut event_clear_level: EventWriter<ClearLevelEvent>,
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        event_clear_level.send(ClearLevelEvent);
        commands.insert_resource(NextState(GameState::MainMenu));
    }
}

pub fn despawn_entities<T: Component>(commands: &mut Commands, query: Query<Entity, With<T>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn despawn_with_system<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    despawn_entities(&mut commands, query);
}

pub fn run(app: &mut App, args: &Args) {
    let start_state = if args.command.is_none() && args.level.is_none() && args.test_level.is_none()
    {
        GameState::MainMenu
    } else if matches!(args.command, Some(args::Commands::Editor)) {
        GameState::Editor
    } else {
        GameState::Game
    };

    app.insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "CatSnake".to_string(),
                        width: 1080.0,
                        height: 720.0,
                        ..default()
                    },
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes: true,
                    ..Default::default()
                }),
        )
        .add_loopless_state_before_stage(CoreStage::PreUpdate, start_state)
        .add_plugin(MenuPlugin)
        .add_plugin(MainMenuPlugin)
        .add_plugin(SelectLevelMenuPlugin)
        .add_plugin(GamePlugin { args: args.clone() })
        .add_plugin(AudioPlugin)
        .add_startup_system(load_assets)
        .run();
}
