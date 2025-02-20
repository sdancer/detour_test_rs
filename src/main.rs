mod debug_draw_b;
mod net;
mod obj_loader;
mod world;

//use crate::obj_loader::load_obj;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use glam::{Vec2, Vec3, Vec4};
use std::path::PathBuf;

// Import the debug draw implementation and obj loader
use obj_loader::ObjData;

use std::io::{Read, Write};

use std::sync::{Arc, Mutex};

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

struct MitmInfo {
    socket: Option<std::net::TcpStream>,
    curpos: Option<(f32, f32, f32)>,
}

#[derive(Component)]
struct MeshViewer {
    obj_path: Option<PathBuf>,
    walkable_slope_angle: f32,
    needs_update: bool, // Add this field to track when updates are needed
    mitm_info: Arc<MitmInfo>,
}

fn ui_system(
    mut contexts: EguiContexts,
    mut mesh_viewer: Query<&mut MeshViewer>,
    camera_query: Query<(&Transform, &MainCamera, &Camera)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut viewer = mesh_viewer.single_mut();

    let mitm_info = &mut viewer.mitm_info;

    egui::Window::new("Mitm").show(contexts.ctx_mut(), |ui| {
        if ui.button("connect").clicked() {
            //connect tcp here

            let res = std::net::TcpStream::connect("127.0.0.1:9999");
            match res {
                Ok(_) => {
                    let mut socket = res.unwrap();
                    let _ = socket.set_nonblocking(true);
                    let _ = socket.write(&[0, 0, 0, 1, 1]); //initial command, watch mitm
                    (*Arc::get_mut(mitm_info).unwrap()).socket = Some(socket);
                }
                Err(_) => {}
            }
        }
    });

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
        ui.add(
            egui::Slider::new(&mut viewer.walkable_slope_angle, 0.0..=90.0)
                .text("Walkable Slope Angle"),
        );

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

    egui::Window::new("Coordinates").show(contexts.ctx_mut(), |ui| {
        if let Ok((transform, camera, camera_comp)) = camera_query.get_single() {
            let pos = transform.translation;
            ui.label(format!(
                "Camera Position: {:.2}, {:.2}, {:.2}",
                pos.x, pos.y, pos.z
            ));

            // Convert radians to degrees for more readable output
            let yaw_degrees = camera.yaw.to_degrees();
            let pitch_degrees = camera.pitch.to_degrees();
            ui.label(format!(
                "Camera Angles: Yaw {:.1}°, Pitch {:.1}°",
                yaw_degrees, pitch_degrees
            ));

            if let Ok(window) = windows.get_single() {
                if let Some(cursor_pos) = window.cursor_position() {
                    ui.label(format!(
                        "Screen Position: {:.2}, {:.2}",
                        cursor_pos.x, cursor_pos.y
                    ));

                    if let Some(world_pos) =
                        world::screen_to_world(window, camera_comp, transform, cursor_pos)
                    {
                        ui.label(format!(
                            "World Position: {:.2}, {:.2}, {:.2}",
                            world_pos.x, world_pos.y, world_pos.z
                        ));
                    }
                }
            }
        }
    });
}

#[derive(Component)]
struct DebugMesh;

#[derive(Component)]
struct TileMesh {
    tile_x: i32,
    tile_y: i32,
}

#[derive(Resource)]
struct MeshData {
    vertices: Vec<Vec3>,
    indices: Vec<u32>,
    normals: Vec<Vec3>,
    tile_size: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_control, ui_system, update_mesh))
        .run();
}

fn split_mesh_into_tiles(
    vertices: &[Vec3],
    indices: &[u32],
    normals: &[Vec3],
    tile_size: f32,
) -> Vec<(i32, i32, Vec<Vec3>, Vec<u32>, Vec<Vec3>)> {
    let mut tiles = Vec::new();
    let mut tile_map: std::collections::HashMap<(i32, i32), (Vec<Vec3>, Vec<u32>, Vec<Vec3>)> =
        std::collections::HashMap::new();

    // Process each triangle
    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let v0 = vertices[triangle[0] as usize];
        let v1 = vertices[triangle[1] as usize];
        let v2 = vertices[triangle[2] as usize];

        // Calculate which tile(s) this triangle belongs to
        let tile_x0 = (v0.x / tile_size).floor() as i32;
        let tile_z0 = (v0.z / tile_size).floor() as i32;
        let tile_x1 = (v1.x / tile_size).floor() as i32;
        let tile_z1 = (v1.z / tile_size).floor() as i32;
        let tile_x2 = (v2.x / tile_size).floor() as i32;
        let tile_z2 = (v2.z / tile_size).floor() as i32;

        // Get the range of tiles this triangle might touch
        let min_tile_x = tile_x0.min(tile_x1).min(tile_x2);
        let max_tile_x = tile_x0.max(tile_x1).max(tile_x2);
        let min_tile_z = tile_z0.min(tile_z1).min(tile_z2);
        let max_tile_z = tile_z0.max(tile_z1).max(tile_z2);

        // Add the triangle to each tile it belongs to
        for tile_x in min_tile_x..=max_tile_x {
            for tile_z in min_tile_z..=max_tile_z {
                let tile_entry = tile_map
                    .entry((tile_x, tile_z))
                    .or_insert_with(|| (Vec::new(), Vec::new(), Vec::new()));

                // Add vertices and update indices
                let base_index = tile_entry.0.len() as u32;
                tile_entry.0.push(v0);
                tile_entry.0.push(v1);
                tile_entry.0.push(v2);
                tile_entry
                    .1
                    .extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
                tile_entry.2.extend_from_slice(&[
                    normals[triangle[0] as usize],
                    normals[triangle[1] as usize],
                    normals[triangle[2] as usize],
                ]);
            }
        }
    }

    // Convert the HashMap into a Vec
    for ((tile_x, tile_z), (verts, inds, norms)) in tile_map {
        tiles.push((tile_x, tile_z, verts, inds, norms));
    }

    tiles
}

// Add this function to calculate colors based on slope
fn calculate_colors(
    vertices: &[Vec3],
    indices: &[u32],
    normals: &[Vec3],
    walkable_slope_angle: f32,
) -> Vec<[f32; 4]> {
    let mut colors = vec![[1.0, 1.0, 1.0, 1.0]; vertices.len()];
    let walkable_thr = (walkable_slope_angle.to_radians()).cos();

    // Unwalkable color (orange: 192,128,0)
    let unwalkable = [192.0 / 255.0, 128.0 / 255.0, 0.0, 1.0];

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
                    1.0,
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
            transform: Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                far: 100000.0,
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
    let water_size = 100000.0; // Large enough to cover the viewable area
                               //let water_plane = shape::Plane::from_size(water_size);
                               //commands.spawn(PbrBundle {
                               //    mesh: meshes.add(water_plane.into()),
                               //    material: materials.add(StandardMaterial {
                               //        base_color: Color::rgba(0.2, 0.4, 0.8, 0.8), // Bluish color with some transparency
                               //        alpha_mode: AlphaMode::Blend,
                               //        unlit: true, // Make it unaffected by lighting
                               //        ..default()
                               //    }),
                               //    transform: Transform::from_xyz(0.0, -1000.0, 0.0), // Position it below the terrain
                               //    ..default()
                               //});
                               // Light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Create default mesh
    let mut default_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let vertices = vec![
        Vec3::new(1.0, 0.0, 1.0),     // Bottom-left corner (0)
        Vec3::new(987.0, 0.0, 1.0),   // Bottom-right corner (1)
        Vec3::new(987.0, 0.0, 987.0), // Top-right corner (2)
        Vec3::new(1.0, 0.0, 987.0),   // Top-left corner (3)
    ];

    let indices = vec![
        0, 1, 2, // First triangle: bottom-left -> bottom-right -> top-right
        0, 2, 3, // Second triangle: bottom-left -> top-right -> top-left
    ];
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
        mitm_info: Arc::new(MitmInfo {
            socket: None,
            curpos: None,
        }),
    });

    // Insert initial mesh data
    commands.insert_resource(MeshData {
        vertices,
        indices,
        normals,
        tile_size: 988.,
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
    mut query: Query<(
        &mut Transform,
        &mut MainCamera,
        &mut Projection,
        &mut CameraMouseState,
    )>,
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
    let ctrl_pressed =
        key_mods.pressed(KeyCode::ControlLeft) || key_mods.pressed(KeyCode::ControlRight);

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
    )
    .normalize();

    let right = forward.cross(Vec3::Y).normalize();
    let up = Vec3::Y;

    // Handle movement
    let mut movement = Vec3::ZERO;
    let move_speed = 988.0 * time.delta_seconds();

    if keyboard.pressed(KeyCode::W) {
        movement += forward;
    }
    if keyboard.pressed(KeyCode::S) {
        movement -= forward;
    }
    if keyboard.pressed(KeyCode::A) {
        movement -= right;
    }
    if keyboard.pressed(KeyCode::D) {
        movement += right;
    }
    if keyboard.pressed(KeyCode::E) {
        movement += up;
    }
    if keyboard.pressed(KeyCode::Q) {
        movement -= up;
    }

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
            transform.translation = Vec3::new(center.x, center.y + 988.0, center.z);

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
    tiles_query: Query<Entity, With<TileMesh>>,
) {
    let mut viewer = mesh_viewer.single_mut();

    if !viewer.needs_update {
        return;
    }

    // Clean up existing tiles
    for entity in tiles_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(path) = &viewer.obj_path {
        if let Ok(obj_data) = obj_loader::load_obj(path) {
            let (vertices, indices, normals) = convert_obj_to_mesh_data(&obj_data);

            // Split into tiles
            let tile_size = 988.0;
            let tiles = split_mesh_into_tiles(&vertices, &indices, &normals, tile_size);

            // Create a mesh for each tile
            for (tile_x, tile_z, tile_vertices, tile_indices, tile_normals) in tiles {
                let colors = calculate_colors(
                    &tile_vertices,
                    &tile_indices,
                    &tile_normals,
                    viewer.walkable_slope_angle,
                );

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, tile_vertices);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, tile_normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                mesh.set_indices(Some(Indices::U32(tile_indices)));

                let material = StandardMaterial {
                    base_color: Color::WHITE,
                    unlit: true,
                    emissive: Color::WHITE,
                    ..default()
                };

                // Spawn a new entity for this tile
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: materials.add(material),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0),
                        ..default()
                    },
                    TileMesh {
                        tile_x,
                        tile_y: tile_z,
                    },
                ));
            }

            commands.insert_resource(MeshData {
                vertices,
                indices,
                normals,
                tile_size,
            });
        }
    }

    viewer.needs_update = false;
}
fn convert_obj_to_mesh_data(obj: &ObjData) -> (Vec<Vec3>, Vec<u32>, Vec<Vec3>) {
    let vertices: Vec<Vec3> = obj
        .vertices
        .iter()
        .skip(1) // Skip the first vertex (0-indexed)
        .map(|v| Vec3::new(v.x, v.y, v.z))
        .collect();

    let triangles = obj.triangulate();
    let indices: Vec<u32> = triangles
        .iter()
        .flat_map(|tri| {
            vec![
                (tri[0] - 1) as u32,
                (tri[1] - 1) as u32,
                (tri[2] - 1) as u32,
            ]
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
