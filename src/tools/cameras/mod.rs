pub mod camera_2d_panzoom;
pub mod camera_3d_free;
pub mod camera_3d_panorbit;

use bevy::render::camera::RenderTarget;
use bevy::utils::HashSet;
use bevy::{prelude::*, render::primitives::Aabb};
// use bevy_editor_pls_core::{
//     editor_window::{EditorWindow, EditorWindowContext},
//     Editor, EditorEvent, EditorState,
// };
use bevy_inspector_egui::egui;
// use bevy_mod_picking::prelude::PickRaycastSource;

// use crate::hierarchy::{HideInEditor, HierarchyWindow};

use self::camera_3d_panorbit::PanOrbitCamera;

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

pub struct CameraWindow;

#[derive(Clone, Copy, PartialEq)]
pub enum EditorCamKind {
    D2PanZoom,
    D3Free,
    D3PanOrbit,
}

impl EditorCamKind {
    fn name(self) -> &'static str {
        match self {
            EditorCamKind::D2PanZoom => "2D (Pan/Zoom)",
            EditorCamKind::D3Free => "3D (Free)",
            EditorCamKind::D3PanOrbit => "3D (Pan/Orbit)",
        }
    }

    fn all() -> [EditorCamKind; 3] {
        [
            EditorCamKind::D2PanZoom,
            EditorCamKind::D3Free,
            EditorCamKind::D3PanOrbit,
        ]
    }
}

impl Default for EditorCamKind {
    fn default() -> Self {
        EditorCamKind::D3PanOrbit
    }
}

#[derive(Default)]
pub struct CameraWindowState {
    // make sure to keep the `ActiveEditorCamera` marker component in sync with this field
    editor_cam: EditorCamKind,
    pub show_ui: bool,
}

impl CameraWindowState {
    pub fn editor_cam(&self) -> EditorCamKind {
        self.editor_cam
    }
}

// impl EditorWindow for CameraWindow {
//     type State = CameraWindowState;

//     const NAME: &'static str = "Cameras";

//     fn ui(world: &mut World, _cx: EditorWindowContext, ui: &mut egui::Ui) {
//         cameras_ui(ui, world);
//     }

//     fn viewport_toolbar_ui(world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
//         let state = cx.state_mut::<CameraWindow>().unwrap();
//         ui.menu_button(state.editor_cam.name(), |ui| {
//             for camera in EditorCamKind::all() {
//                 ui.horizontal(|ui| {
//                     if ui.button(camera.name()).clicked() {
//                         if state.editor_cam != camera {
//                             set_active_editor_camera_marker(world, camera);
//                         }

//                         state.editor_cam = camera;

//                         ui.close_menu();
//                     }
//                 });
//             }
//         });
//         ui.checkbox(&mut state.show_ui, "UI");
//     }

//     fn app_setup(app: &mut App) {
//         app.init_resource::<PreviouslyActiveCameras>();

//         app.add_plugin(camera_2d_panzoom::PanCamPlugin)
//             .add_plugin(camera_3d_free::FlycamPlugin)
//             .add_plugin(camera_3d_panorbit::PanOrbitCameraPlugin)
//             .add_system(
//                 set_editor_cam_active
//                     .before(camera_3d_panorbit::CameraSystem::Movement)
//                     .before(camera_3d_free::CameraSystem::Movement)
//                     .before(camera_2d_panzoom::CameraSystem::Movement),
//             )
//             .add_system_to_stage(CoreStage::PreUpdate, toggle_editor_cam)
//             .add_system_to_stage(CoreStage::PreUpdate, focus_selected)
//             .add_system(initial_camera_setup);
//         app.add_startup_system_to_stage(StartupStage::PreStartup, spawn_editor_cameras);

//         /*app.add_system_to_stage(
//             CoreStage::PostUpdate,
//             set_main_pass_viewport.before(bevy::render::camera::CameraUpdateSystem),
//         );*/
//     }
// }

fn set_active_editor_camera_marker(world: &mut World, editor_cam: EditorCamKind) {
    let mut previously_active = world.query_filtered::<Entity, With<ActiveEditorCamera>>();
    let mut previously_active_iter = previously_active.iter(world);
    let previously_active = previously_active_iter.next();

    assert!(
        previously_active_iter.next().is_none(),
        "there should be only one `ActiveEditorCamera`"
    );

    if let Some(previously_active) = previously_active {
        world
            .entity_mut(previously_active)
            .remove::<ActiveEditorCamera>();
    }

    let entity = match editor_cam {
        EditorCamKind::D2PanZoom => {
            let mut state = world.query_filtered::<Entity, With<EditorCamera2dPanZoom>>();
            state.iter(world).next().unwrap()
        }
        EditorCamKind::D3Free => {
            let mut state = world.query_filtered::<Entity, With<EditorCamera3dFree>>();
            state.iter(world).next().unwrap()
        }
        EditorCamKind::D3PanOrbit => {
            let mut state = world.query_filtered::<Entity, With<EditorCamera3dPanOrbit>>();
            state.iter(world).next().unwrap()
        }
    };
    world.entity_mut(entity).insert(ActiveEditorCamera);
}

fn spawn_editor_cameras(mut commands: Commands) {
    #[derive(Component, Default)]
    struct Ec2d;
    #[derive(Component, Default)]
    struct Ec3d;

    info!("Spawning editor cameras");

    let show_ui_by_default = false;
    let editor_cam_priority = 100;

    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                priority: editor_cam_priority,
                is_active: false,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 2.0, 5.0),
            ..Camera3dBundle::default()
        })
        .insert(UiCameraConfig {
            show_ui: show_ui_by_default,
        })
        .insert(Ec3d)
        .insert(camera_3d_free::FlycamControls::default())
        .insert(EditorCamera)
        .insert(EditorCamera3dFree)
        .insert(Name::new("Editor Camera 3D Free"));
}

#[derive(Resource, Default)]
struct PreviouslyActiveCameras(HashSet<Entity>);

/*fn set_main_pass_viewport(
    editor_state: Res<bevy_editor_pls_core::EditorState>,
    egui_settings: Res<bevy_inspector_egui::bevy_egui::EguiSettings>,
    windows: Res<Windows>,
    mut cameras: Query<&mut Camera>,
) {
    if !editor_state.is_changed() {
        return;
    };

    let scale_factor = windows.get_primary().unwrap().scale_factor() * egui_settings.scale_factor;

    let viewport_pos = editor_state.viewport.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = editor_state.viewport.size() * scale_factor as f32;

    cameras.iter_mut().for_each(|mut cam| {
        cam.viewport = editor_state.active.then(|| bevy::render::camera::Viewport {
            physical_position: UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32),
            physical_size: UVec2::new(
                (viewport_size.x as u32).max(1),
                (viewport_size.y as u32).max(1),
            ),
            depth: 0.0..1.0,
        });
    });
}*/
