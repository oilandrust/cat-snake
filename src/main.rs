use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use dev_tools_pluggin::DevToolsPlugin;
use game_constants_pluggin::*;
use level_pluggin::{LevelPluggin, StartLevelEvent};
use movement_pluggin::MovementPluggin;
use snake_pluggin::SnakePluggin;

mod commands;
mod dev_tools_pluggin;
mod game_constants_pluggin;
mod level_pluggin;
mod level_template;
mod levels;
mod movement_pluggin;
mod snake_pluggin;
mod undo;

fn start_game(mut event_writer: EventWriter<StartLevelEvent>) {
    event_writer.send(StartLevelEvent(7));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(DARK_COLOR_PALETTE[4]))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Snake".to_string(),
                width: 640.0,
                height: 420.0,
                ..default()
            },
            ..default()
        }))
        .add_plugin(TweeningPlugin)
        .add_plugin(GameConstantsPlugin)
        .add_plugin(DevToolsPlugin)
        .add_plugin(SnakePluggin)
        .add_plugin(LevelPluggin)
        .add_plugin(MovementPluggin)
        .add_startup_system(start_game)
        .add_system(bevy::window::close_on_esc)
        .run();
}
