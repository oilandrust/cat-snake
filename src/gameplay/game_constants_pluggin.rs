use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;

pub const MOVE_START_VELOCITY: f32 = 5.0;
pub const JUMP_START_VELOCITY: f32 = 65.0;
pub const GRAVITY: f32 = 30.0;

pub const UP: IVec3 = IVec3::Y;
pub const DOWN: IVec3 = IVec3::NEG_Y;
pub const RIGHT: IVec3 = IVec3::X;
pub const LEFT: IVec3 = IVec3::NEG_X;

macro_rules! rgb_u8 {
    ($r:expr, $g:expr, $b:expr) => {
        Color::rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

macro_rules! rgba_u8 {
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        Color::rgba(
            $r as f32 / 255.0,
            $g as f32 / 255.0,
            $b as f32 / 255.0,
            $a as f32 / 255.0,
        )
    };
}

pub const BACKGROUND_COLOR: Color = rgb_u8!(204, 217, 255);
pub const SPIKE_COLOR: Color = Color::rgb(0.8, 0.7176471, 0.68235296);
pub const WALL_COLOR: Color = rgb_u8!(119, 89, 54);
pub const WATER_COLOR: Color = rgba_u8!(27, 85, 124, 108);
pub const FOOD_COLOR: Color = Color::rgb(0.9764706, 0.5176471, 0.2901961);

pub const SNAKE_COLORS: [[Color; 2]; 3] = [
    [
        rgb_u8!(68, 171, 96),
        Color::rgb(0.5647059, 0.74509805, 0.42745098),
    ],
    [
        Color::rgb(0.972549, 0.5113725, 0.0),
        Color::rgb(0.972549, 0.5882353, 0.11764706),
    ],
    [rgb_u8!(66, 135, 245), rgb_u8!(105, 159, 245)],
];

pub fn to_world(position: IVec3) -> Vec3 {
    position.as_vec3() + 0.5
}

pub fn to_grid(position: Vec3) -> IVec3 {
    (position - 0.5).round().as_ivec3()
}

#[derive(Resource, Reflect, InspectorOptions)]
#[reflect(InspectorOptions)]
pub struct GameConstants {
    #[inspector(min = 0.0, max = 300.0)]
    pub move_velocity: f32,

    #[inspector(min = 0.0, max = 300.0)]
    pub jump_velocity: f32,

    #[inspector(min = 0.0, max = 900.0)]
    pub gravity: f32,

    pub background_color: Color,

    pub ground_color: Color,

    pub water_color: Color,
}

impl Default for GameConstants {
    fn default() -> Self {
        Self {
            move_velocity: MOVE_START_VELOCITY,
            jump_velocity: JUMP_START_VELOCITY,
            gravity: GRAVITY,
            background_color: BACKGROUND_COLOR,
            ground_color: WALL_COLOR,
            water_color: WATER_COLOR,
        }
    }
}
pub struct GameConstantsPlugin;

impl Plugin for GameConstantsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GameConstants>()
            .insert_resource(GameConstants::default())
            .add_system(update_colors);
    }
}

fn update_colors(mut commands: Commands, game_constants: Res<GameConstants>) {
    commands.insert_resource(ClearColor(game_constants.background_color));
}
