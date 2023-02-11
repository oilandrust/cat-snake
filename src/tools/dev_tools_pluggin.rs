use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::bevy_inspector;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_prototype_debug_lines::DebugLinesPlugin;
use iyes_loopless::prelude::ConditionSet;

use crate::gameplay::game_constants_pluggin::GameConstants;
use crate::GameState;

pub struct DevToolsPlugin;

#[derive(Default, Resource)]
pub struct DevToolsSettings {
    pub dev_tools_enabled: bool,
    pub inspector_enabled: bool,
}

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DevToolsSettings>()
            // .add_plugin(LogDiagnosticsPlugin::default())
            // .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(DebugLinesPlugin::default())
            .add_plugin(EguiPlugin)
            .add_plugin(DefaultInspectorConfigPlugin)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Game)
                    .with_system(toogle_dev_tools_system)
                    .with_system(inspector_ui_system)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Editor)
                    .with_system(toogle_dev_tools_system)
                    .with_system(inspector_ui_system)
                    .into(),
            );
    }
}

fn toogle_dev_tools_system(
    keyboard: Res<Input<KeyCode>>,
    mut dev_tool_settings: ResMut<DevToolsSettings>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        let old_value = dev_tool_settings.dev_tools_enabled;
        dev_tool_settings.dev_tools_enabled = !old_value;
    }

    if keyboard.just_pressed(KeyCode::I) {
        let old_value = dev_tool_settings.inspector_enabled;
        dev_tool_settings.inspector_enabled = !old_value;
    }
}

fn inspector_ui_system(world: &mut World) {
    let dev_tool_settings = world
        .get_resource::<DevToolsSettings>()
        .expect("A dev tools settings resource should be present.");

    if !dev_tool_settings.dev_tools_enabled {
        return;
    }

    if !dev_tool_settings.inspector_enabled {
        return;
    }

    let egui_context = world
        .resource_mut::<bevy_egui::EguiContext>()
        .ctx_mut()
        .clone();

    egui::Window::new("GameConstants").show(&egui_context, |ui| {
        bevy_inspector_egui::bevy_inspector::ui_for_resource::<GameConstants>(world, ui);
    });

    egui::Window::new("Inspector").show(&egui_context, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            bevy_inspector::ui_for_world(world, ui);
        });
    });
}
