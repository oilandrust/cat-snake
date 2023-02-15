pub mod camera_2d_panzoom;
pub mod camera_3d_free;
pub mod camera_3d_panorbit;

use bevy::prelude::*;
// use bevy_editor_pls_core::{
//     editor_window::{EditorWindow, EditorWindowContext},
//     Editor, EditorEvent, EditorState,
// };

// Present on all editor cameras
#[derive(Component)]
pub struct EditorCamera;

// Present only one the one currently active camera
#[derive(Component)]
pub struct ActiveEditorCamera;

// Marker component for the 3d free camera
#[derive(Component)]
struct EditorCamera3dFree;

// Marker component for the 3d pan+orbit
#[derive(Component)]
struct EditorCamera3dPanOrbit;

// Marker component for the 2d pan+zoom camera
#[derive(Component)]
struct EditorCamera2dPanZoom;
