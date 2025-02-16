use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use glam::{Vec2, Vec3, Vec4};
use std::path::PathBuf;
use bevy::input::mouse::MouseMotion;
use crate::obj_loader::load_obj;

// Import the debug draw implementation and obj loader
mod obj_loader;
use obj_loader::{self, ObjData};

// Components
#[derive(Component)]
struct MainCamera {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
struct MeshViewer {
    obj_path: Option<PathBuf>,
    walkable_slope_angle: f32,
}

#[derive(Component)]
struct DebugMesh;

// Resources
#[derive(Resource)]
struct MeshData {
    vertices: Vec<Vec3>,
    indices: Vec<u32>,
    normals: Vec<Vec3>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_control,
            ui_system,
            update_mesh,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 2.0, 5.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera {
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
        },
    ));

    // Light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Create default mesh
    let mut default_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let vertices = vec![
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    let normal1 = (vertices[1] - vertices[0])
        .cross(vertices[2] - vertices[0])
        .normalize();
    let normal2 = (vertices[2] - vertices[0])
        .cross(vertices[3] - vertices[0])
        .normalize();
    let normals = vec![normal1, normal1, normal1, normal2, normal2, normal2];

    default_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
    default_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
    default_mesh.set_indices(Some(Indices::U32(indices.clone())));

    // Spawn mesh entity
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(default_mesh),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.8, 0.8),
                ..default()
            }),
            ..default()
        },
        DebugMesh,
    ));

    // Spawn mesh viewer
    commands.spawn(MeshViewer {
        obj_path: None,
        walkable_slope_angle: 45.0,
    });

    // Insert initial mesh data
    commands.insert_resource(MeshData {
        vertices,
        indices,
        normals,
    });
}

fn camera_control(
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mouse_button: Res<Input<MouseButton>>,
    mut query: Query<(&mut Transform, &mut MainCamera)>,
) {
    let (mut transform, mut camera) = query.single_mut();
    
    // Handle rotation
    if mouse_button.pressed(MouseButton::Right) {
        for ev in mouse_motion.iter() {
            camera.yaw += ev.delta.x * 0.005;
            camera.pitch = (camera.pitch - ev.delta.y * 0.005)
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }
    }

    // Calculate movement vectors
    let forward = Vec3::new(
        camera.yaw.cos() * camera.pitch.cos(),
        camera.pitch.sin(),
        camera.yaw.sin() * camera.pitch.cos(),
    ).normalize();
    
    let right = forward.cross(Vec3::Y).normalize();
    let up = Vec3::Y;

    // Handle movement
    let mut movement = Vec3::ZERO;
    let move_speed = 5.0 * time.delta_seconds();

    if keyboard.pressed(KeyCode::W) { movement += forward; }
    if keyboard.pressed(KeyCode::S) { movement -= forward; }
    if keyboard.pressed(KeyCode::A) { movement -= right; }
    if keyboard.pressed(KeyCode::D) { movement += right; }
    if keyboard.pressed(KeyCode::E) { movement += up; }
    if keyboard.pressed(KeyCode::Q) { movement -= up; }

    transform.translation += movement * move_speed;
    transform.look_to(forward, Vec3::Y);
}

fn ui_system(
    mut contexts: EguiContexts,
    mut mesh_viewer: Query<&mut MeshViewer>,
) {
    let mut viewer = mesh_viewer.single_mut();

    egui::Window::new("Mesh Viewer Controls").show(contexts.ctx_mut(), |ui| {
        // File loading button
        if ui.button("Load OBJ").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OBJ files", &["obj"])
                .pick_file()
            {
                viewer.obj_path = Some(path);
            }
        }

        // Display loaded file path
        if let Some(path) = &viewer.obj_path {
            ui.label(format!("Loaded: {}", path.display()));
        }

        // Walkable slope angle slider
        ui.add(egui::Slider::new(&mut viewer.walkable_slope_angle, 0.0..=90.0)
            .text("Walkable Slope Angle"));

        // Controls help
        ui.separator();
        ui.label("Controls:");
        ui.label("WASD - Move");
        ui.label("Q/E - Up/Down");
        ui.label("Right Click + Drag - Look");
    });
}

fn update_mesh(
    mut commands: Commands,
    mesh_viewer: Query<&MeshViewer, Changed<MeshViewer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_query: Query<&mut Handle<Mesh>, With<DebugMesh>>,
) {
    for viewer in mesh_viewer.iter() {
        if let Some(path) = &viewer.obj_path {
            if let Ok(obj_data) = obj_loader::load_obj(path) {
                let (vertices, indices, normals) = convert_obj_to_mesh_data(&obj_data);
                
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
                mesh.set_indices(Some(Indices::U32(indices.clone())));

                // Update mesh handle
                if let Ok(mesh_handle) = mesh_query.get_single_mut() {
                    *mesh_handle = meshes.add(mesh);
                }

                // Update mesh data resource
                commands.insert_resource(MeshData {
                    vertices,
                    indices,
                    normals,
                });
            }
        }
    }
}

fn convert_obj_to_mesh_data(obj: &ObjData) -> (Vec<Vec3>, Vec<u32>, Vec<Vec3>) {
    let vertices: Vec<Vec3> = obj.vertices.iter()
        .skip(1)  // Skip the first vertex (0-indexed)
        .map(|v| Vec3::new(v.x, v.y, v.z))
        .collect();

    let triangles = obj.triangulate();
    let indices: Vec<u32> = triangles.iter()
        .flat_map(|tri| {
            vec![(tri[0] - 1) as u32, (tri[1] - 1) as u32, (tri[2] - 1) as u32]
        })
        .collect();

    // Calculate normals
    let mut normals = Vec::new();
    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let v0 = vertices[chunk[0] as usize];
            let v1 = vertices[chunk[1] as usize];
            let v2 = vertices[chunk[2] as usize];
            let normal = (v1 - v0).cross(v2 - v0).normalize();
            normals.extend_from_slice(&[normal, normal, normal]);
        }
    }

    (vertices, indices, normals)
}
