use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::bevy_inspector;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use iyes_loopless::prelude::ConditionSet;

use crate::gameplay::game_constants_pluggin::to_world;
use crate::gameplay::game_constants_pluggin::GameConstants;
use crate::gameplay::level_pluggin::LevelEntity;
use crate::level::level_instance::LevelEntityType;
use crate::level::level_instance::LevelInstance;
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
                    .run_in_state(GameState::Game)
                    .run_if_resource_exists::<LevelInstance>()
                    .with_system(draw_transforms_system)
                    .with_system(debug_draw_level_cells)
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

pub fn draw_cross(lines: &mut DebugLines, position: Vec3, color: Color) {
    lines.line_colored(
        position + Vec3::new(0.5, 0.5, 0.0),
        position + Vec3::new(-0.5, -0.5, 0.0),
        0.,
        color,
    );

    lines.line_colored(
        position + Vec3::new(-0.5, 0.5, 0.0),
        position + Vec3::new(0.5, -0.5, 0.0),
        0.,
        color,
    );
}

fn debug_draw_level_cells(
    dev_tool_settings: Res<DevToolsSettings>,
    mut lines: ResMut<DebugLines>,
    level: Res<LevelInstance>,
) {
    if !dev_tool_settings.dev_tools_enabled {
        return;
    }

    for (position, value) in level.occupied_cells() {
        let world_grid = to_world(*position);
        let world_grid = Vec3::new(world_grid.x, world_grid.y, 0.0);

        let color = match value {
            LevelEntityType::Food => Color::RED,
            LevelEntityType::Wall => Color::BLACK,
            LevelEntityType::Snake(_) => Color::BLUE,
            LevelEntityType::Spike => Color::DARK_GRAY,
        };

        draw_cross(lines.as_mut(), world_grid, color);
    }
}

fn draw_transforms_system(
    dev_tool_settings: Res<DevToolsSettings>,
    mut lines: ResMut<DebugLines>,
    query: Query<&GlobalTransform, With<LevelEntity>>,
) {
    if !dev_tool_settings.dev_tools_enabled {
        return;
    }

    for transform in &query {
        lines.line_colored(
            transform.translation(),
            transform.translation() + 2.0 * transform.right(),
            0.,
            Color::RED,
        );

        lines.line_colored(
            transform.translation(),
            transform.translation() + 2.0 * transform.up(),
            0.,
            Color::GREEN,
        );

        lines.line_colored(
            transform.translation(),
            transform.translation() + 2.0 * transform.back(),
            0.,
            Color::BLUE,
        );
    }
}
