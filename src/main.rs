mod debug_draw_b;
mod obj_loader;

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use glam::{Vec2, Vec3};
use std::path::PathBuf;
use bevy::input::mouse::MouseMotion;
use bevy::render::render_asset::{RenderAssetUsages};
use native_dialog::FileDialog;

// Components
#[derive(Component)]
struct MainCamera {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
struct CameraMouseState {
    initial_position: Option<Vec2>,
    last_position: Option<Vec2>,
}

#[derive(Component)]
struct MeshHandle(Handle<Mesh>);

impl Default for CameraMouseState {
    fn default() -> Self {
        Self {
            initial_position: None,
            last_position: None,
        }
    }
}

#[derive(Component)]
struct MeshViewer {
    obj_path: Option<PathBuf>,
    walkable_slope_angle: f32,
    needs_update: bool,
}

fn ui_system(
    mut contexts: EguiContexts,
    mut mesh_viewer: Query<&mut MeshViewer>,
) {
    let mut viewer = mesh_viewer.single_mut();

    egui::Window::new("Mesh Viewer Controls").show(contexts.ctx_mut(), |ui| {
        
     if ui.button("Load OBJ").clicked() {
    match FileDialog::new()
        .set_location(&std::env::current_dir().unwrap_or_default())
        .add_filter("OBJ Files", &["obj"])
        .show_open_single_file()
    {
        Ok(Some(path)) => {
            viewer.obj_path = Some(path);
            viewer.needs_update = true;
        }
        Ok(None) => {
            println!("File selection cancelled");
        }
        Err(e) => {
            println!("Error opening file dialog: {}", e);
        }
    }
}


        if let Some(path) = &viewer.obj_path {
            ui.label(format!("Loaded: {}", path.display()));
        }

        let prev_angle = viewer.walkable_slope_angle;
        ui.add(egui::Slider::new(&mut viewer.walkable_slope_angle, 0.0..=90.0)
            .text("Walkable Slope Angle"));
        
        if (viewer.walkable_slope_angle - prev_angle).abs() > f32::EPSILON {
            viewer.needs_update = true;
        }

        ui.separator();
        ui.label("Controls:");
        ui.label("WASD - Move");
        ui.label("Q/E - Up/Down");
        ui.label("Right Click + Drag - Look");
    });
}

#[derive(Component)]
struct DebugMesh;

#[derive(Resource)]
struct MeshData {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_control,
            ui_system,
            update_mesh,
        ))
        .run();
}

fn calculate_colors(vertices: &[[f32; 3]], indices: &[u32], normals: &[[f32; 3]], walkable_slope_angle: f32) -> Vec<[f32; 4]> {
    let mut colors = vec![[1.0, 1.0, 1.0, 1.0]; vertices.len()];
    let walkable_thr = walkable_slope_angle.to_radians().cos();
    let unwalkable = [192.0/255.0, 128.0/255.0, 0.0, 1.0];

    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let normal = Vec3::from_array(normals[chunk[0] as usize]);
            let brightness = (220.0 * (2.0 + normal.x + normal.y) / 4.0) / 255.0;
            let grey = [brightness, brightness, brightness, 1.0];

            let color = if normal.y < walkable_thr {
                let t = 64.0 / 255.0;
                [
                    grey[0] * (1.0 - t) + unwalkable[0] * t,
                    grey[1] * (1.0 - t) + unwalkable[1] * t,
                    grey[2] * (1.0 - t) + unwalkable[2] * t,
                    1.0
                ]
            } else {
                grey
            };

            for &index in chunk {
                colors[index as usize] = color;
            }
        }
    }
    colors
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 2.0, 5.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                far: 1000.0,
                near: 0.01,
                fov: 60.0_f32.to_radians(),
                ..default()
            }),
            ..default()
        },
        MainCamera {
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
        },
        CameraMouseState::default(),
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let vertices = vec![
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let colors = calculate_colors(&vertices, &indices, &normals, 45.0);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices.clone()));

    commands.spawn((
        Mesh3d(meshes.add(mesh.clone())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        })),
        MeshHandle(meshes.add(mesh)),
        DebugMesh,
    ));

    commands.spawn(MeshViewer {
        obj_path: None,
        walkable_slope_angle: 45.0,
        needs_update: false,
    });

    commands.insert_resource(MeshData {
        vertices,
        indices,
        normals,
    });
}

fn camera_control(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mesh_data: Option<Res<MeshData>>,
    mut query: Query<(&mut Transform, &mut MainCamera, &mut Projection, &mut CameraMouseState)>,
) {
    let (mut transform, mut camera, mut projection, mut mouse_state) = query.single_mut();
    let window = windows.single();

    if let Projection::Perspective(ref mut perspective) = *projection {
        perspective.fov = 60.0_f32.to_radians();
        perspective.near = 0.01;
        perspective.far = 1000.0;
    }

    if mouse_button.just_pressed(MouseButton::Right) {
        if let Some(position) = window.cursor_position() {
            mouse_state.initial_position = Some(position);
            mouse_state.last_position = Some(position);
        }
    }

    if mouse_button.just_released(MouseButton::Right) {
        mouse_state.initial_position = None;
        mouse_state.last_position = None;
    }

    if mouse_button.pressed(MouseButton::Right) {
        for ev in mouse_motion.read() {
            if let Some(last_pos) = mouse_state.last_position {
                if let Some(current_pos) = window.cursor_position() {
                    let delta = current_pos - last_pos;
                    camera.yaw += delta.x * 0.00125;
                    let new_pitch = camera.pitch - delta.y * 0.00125;
                    camera.pitch = new_pitch.clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
                    mouse_state.last_position = Some(current_pos);
                }
            }
        }
    }

    let forward = Vec3::new(
        camera.yaw.cos() * camera.pitch.cos(),
        camera.pitch.sin(),
        camera.yaw.sin() * camera.pitch.cos(),
    ).normalize();

    let right = forward.cross(Vec3::Y).normalize();
    let up = Vec3::Y;

    let mut movement = Vec3::ZERO;
    let move_speed = 5.0 * time.delta_secs();

    if keyboard.pressed(KeyCode::KeyW) { movement += forward; }
    if keyboard.pressed(KeyCode::KeyS) { movement -= forward; }
    if keyboard.pressed(KeyCode::KeyA) { movement -= right; }
    if keyboard.pressed(KeyCode::KeyD) { movement += right; }
    if keyboard.pressed(KeyCode::KeyE) { movement += up; }
    if keyboard.pressed(KeyCode::KeyQ) { movement -= up; }

    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Some(mesh_data) = mesh_data {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_z = f32::MAX;
            let mut max_z = f32::MIN;
            let mut center = Vec3::ZERO;

            for vertex in &mesh_data.vertices {
                min_x = min_x.min(vertex[0]);
                max_x = max_x.max(vertex[0]);
                min_z = min_z.min(vertex[2]);
                max_z = max_z.max(vertex[2]);
                center += Vec3::from_array(*vertex);
            }

            center /= mesh_data.vertices.len() as f32;
            let terrain_width = (max_x - min_x).abs();

            transform.translation = Vec3::new(
                center.x,
                center.y + terrain_width * 0.5,
                center.z
            );

            camera.pitch = -45.0_f32.to_radians();
            camera.yaw = -90.0_f32.to_radians();
        }
    }

    transform.translation += movement * move_speed;
    transform.look_to(forward, Vec3::Y);
}

fn update_mesh(
    mut commands: Commands,
    mut mesh_viewer: Query<&mut MeshViewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_query: Query<&mut MeshHandle, With<DebugMesh>>,
) {
    let mut viewer = mesh_viewer.single_mut();
    if !viewer.needs_update {
        return;
    }

    if let Some(path) = &viewer.obj_path {
        if let Ok(obj_data) = obj_loader::load_obj(path) {
            let (vertices, indices, normals) = obj_loader::convert_obj_to_mesh_data(&obj_data);
            let colors = calculate_colors(&vertices, &indices, &normals, viewer.walkable_slope_angle);

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            mesh.insert_indices(Indices::U32(indices.clone()));

            if let Ok(mut mesh_handle) = mesh_query.get_single_mut() {
                mesh_handle.0 = meshes.add(mesh);
            }

            commands.insert_resource(MeshData {
                vertices,
                indices,
                normals,
            });
        }
    }

    viewer.needs_update = false;
}
