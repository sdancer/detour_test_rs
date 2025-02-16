mod debug_draw_b;
mod obj_loader;


use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use glam::{Vec2, Vec3, Vec4};
use std::path::PathBuf;
use bevy::input::mouse::MouseMotion;
use crate::obj_loader::load_obj;

// Import the debug draw implementation and obj loader
use obj_loader::{ObjData};

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
    needs_update: bool, // Add this field to track when updates are needed
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
                viewer.needs_update = true; // Set flag when new file is loaded
            }
        }

        // Display loaded file path
        if let Some(path) = &viewer.obj_path {
            ui.label(format!("Loaded: {}", path.display()));
        }

        // Walkable slope angle slider
        let prev_angle = viewer.walkable_slope_angle;
        ui.add(egui::Slider::new(&mut viewer.walkable_slope_angle, 0.0..=90.0)
            .text("Walkable Slope Angle"));
        
        // Set update flag if angle changed
        if (viewer.walkable_slope_angle - prev_angle).abs() > f32::EPSILON {
            viewer.needs_update = true;
        }

        // Controls help
        ui.separator();
        ui.label("Controls:");
        ui.label("WASD - Move");
        ui.label("Q/E - Up/Down");
        ui.label("Right Click + Drag - Look");
    });
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

// Add this function to calculate colors based on slope
fn calculate_colors(vertices: &[Vec3], indices: &[u32], normals: &[Vec3], walkable_slope_angle: f32) -> Vec<[f32; 4]> {
    let mut colors = vec![[1.0, 1.0, 1.0, 1.0]; vertices.len()];
    let walkable_thr = (walkable_slope_angle.to_radians()).cos();
    
    // Unwalkable color (orange: 192,128,0)
    let unwalkable = [192.0/255.0, 128.0/255.0, 0.0, 1.0];
    
    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let normal = normals[chunk[0] as usize];
            
            // Calculate brightness based on normal x and y components
            let brightness = (220.0 * (2.0 + normal.x + normal.y) / 4.0) / 255.0;
            let grey = [brightness, brightness, brightness, 1.0];
            
            let color = if normal.y < walkable_thr {
                // Lerp between grey and orange for unwalkable surfaces
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
            
            // Apply the color to all vertices of the triangle
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
    // Camera with adjusted settings
    commands.spawn((
        Camera3dBundle {
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
        FogSettings {
            color: Color::rgba(0.1, 0.1, 0.1, 1.0),
            directional_light_color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::Linear { start: 5.0, end: 200.0 },
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
    let colors = calculate_colors(&vertices, &indices, &normals, 45.0);

    default_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
    default_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
    default_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    default_mesh.set_indices(Some(Indices::U32(indices.clone())));

    // Spawn mesh entity
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(default_mesh),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
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
        needs_update: false,
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
    key_mods: Res<Input<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mesh_data: Option<Res<MeshData>>, // Add MeshData as an optional resource
    mut query: Query<(&mut Transform, &mut MainCamera, &mut Projection, &mut CameraMouseState)>,
) {
    let (mut transform, mut camera, mut projection, mut mouse_state) = query.single_mut();
    let window = windows.single();
    
    // Adjust projection if it's perspective
    if let Projection::Perspective(ref mut perspective) = *projection {
        perspective.fov = 60.0_f32.to_radians();
        perspective.near = 0.01;
        perspective.far = 1000.0;
    }
    
    // Handle rotation - using CTRL + Left Click
    let ctrl_pressed = key_mods.pressed(KeyCode::ControlLeft) || key_mods.pressed(KeyCode::ControlRight);
    
    // Track mouse button press/release
    if ctrl_pressed && mouse_button.just_pressed(MouseButton::Left) {
        if let Some(position) = window.cursor_position() {
            mouse_state.initial_position = Some(position);
            mouse_state.last_position = Some(position);
        }
    }
    
    if mouse_button.just_released(MouseButton::Left) {
        mouse_state.initial_position = None;
        mouse_state.last_position = None;
    }
    
    // Handle mouse movement when dragging
    if ctrl_pressed && mouse_button.pressed(MouseButton::Left) {
        for ev in mouse_motion.iter() {
            if let Some(last_pos) = mouse_state.last_position {
                // Update position
                if let Some(current_pos) = window.cursor_position() {
                    let delta = current_pos - last_pos;
                    
                    // Apply camera rotation
                    camera.yaw += delta.x * 0.00125;
                    let new_pitch = camera.pitch - delta.y * 0.00125;
                    camera.pitch = new_pitch.clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
                    
                    // Update last position
                    mouse_state.last_position = Some(current_pos);
                }
            }
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

 if keyboard.just_pressed(KeyCode::F) {
        if let Some(mesh_data) = mesh_data {
            // Calculate terrain bounds
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_z = f32::MAX;
            let mut max_z = f32::MIN;
            let mut center = Vec3::ZERO;

            for vertex in &mesh_data.vertices {
                min_x = min_x.min(vertex.x);
                max_x = max_x.max(vertex.x);
                min_z = min_z.min(vertex.z);
                max_z = max_z.max(vertex.z);
                center += *vertex;
            }

            center /= mesh_data.vertices.len() as f32;
            let terrain_width = (max_x - min_x).abs();
            
            // Position camera above terrain
            transform.translation = Vec3::new(
                center.x,
                center.y + terrain_width * 0.5,
                center.z
            );

            // Update camera angles to look at center
            camera.pitch = -45.0_f32.to_radians(); // Look down at 45 degrees
            camera.yaw = -90.0_f32.to_radians(); // Face forward
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
    mut mesh_query: Query<(&mut Handle<Mesh>, &mut Handle<StandardMaterial>), With<DebugMesh>>,
) {
    let mut viewer = mesh_viewer.single_mut();
    
    // Only process if an update is needed
    if !viewer.needs_update {
        return;
    }

    if let Some(path) = &viewer.obj_path {
        if let Ok(obj_data) = obj_loader::load_obj(path) {
            let (vertices, indices, normals) = convert_obj_to_mesh_data(&obj_data);
            let colors = calculate_colors(&vertices, &indices, &normals, viewer.walkable_slope_angle);
            
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone());
            mesh.set_indices(Some(Indices::U32(indices.clone())));

            // Create new material that uses vertex colors
            let material = StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            };

            // Update mesh and material handles
            if let Ok((mut mesh_handle, mut material_handle)) = mesh_query.get_single_mut() {
                *mesh_handle = meshes.add(mesh);
                *material_handle = materials.add(material);
            }

            // Update mesh data resource
            commands.insert_resource(MeshData {
                vertices,
                indices,
                normals,
            });
        }
    }

    // Reset the update flag
    viewer.needs_update = false;
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

    // Calculate normals per vertex by averaging face normals
    let mut normals = vec![Vec3::ZERO; vertices.len()];
    let mut normal_counts = vec![0; vertices.len()];

    // Calculate face normals and accumulate them for each vertex
    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let v0 = vertices[chunk[0] as usize];
            let v1 = vertices[chunk[1] as usize];
            let v2 = vertices[chunk[2] as usize];
            let normal = (v1 - v0).cross(v2 - v0).normalize();

            // Add the face normal to each vertex's accumulated normal
            for &index in chunk {
                normals[index as usize] += normal;
                normal_counts[index as usize] += 1;
            }
        }
    }

    // Average the normals
    for (normal, count) in normals.iter_mut().zip(normal_counts.iter()) {
        if *count > 0 {
            *normal = (*normal / *count as f32).normalize();
        }
    }

    (vertices, indices, normals)
}
