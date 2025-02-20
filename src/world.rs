use bevy::math::Vec2;
use bevy::math::Vec3;
use bevy::math::Vec4;
use bevy::render::camera::Camera;
use bevy::transform::components::Transform;
use bevy::window::Window;

pub fn screen_to_world(
    window: &Window,
    camera: &Camera,
    camera_transform: &Transform,
    cursor_pos: Vec2,
) -> Option<Vec3> {
    // Get the viewport size
    let viewport_size = Vec2::new(window.width(), window.height());

    // Convert screen coordinates to normalized device coordinates (NDC)
    let ndc = Vec2::new(
        (cursor_pos.x / viewport_size.x) * 2.0 - 1.0,
        -(cursor_pos.y / viewport_size.y) * 2.0 + 1.0,
    );

    // Get the view-projection matrix and its inverse
    let projection = camera.projection_matrix();
    let view_matrix = camera_transform.compute_matrix();
    let view_proj = projection * view_matrix;

    let inverse_view_proj = view_proj.inverse();
    // Create points in NDC space
    let near_ndc = Vec4::new(ndc.x, ndc.y, -1.0, 1.0);
    let far_ndc = Vec4::new(ndc.x, ndc.y, 1.0, 1.0);

    // Transform to world space
    let near_world = inverse_view_proj * near_ndc;
    let far_world = inverse_view_proj * far_ndc;

    // Perspective divide
    let near_point = Vec3::new(
        near_world.x / near_world.w,
        near_world.y / near_world.w,
        near_world.z / near_world.w,
    );
    let far_point = Vec3::new(
        far_world.x / far_world.w,
        far_world.y / far_world.w,
        far_world.z / far_world.w,
    );

    // Calculate ray direction
    let ray_direction = (far_point - near_point).normalize();

    // For simplicity, we'll intersect with the Y=0 plane
    if ray_direction.y.abs() > 0.0001 {
        let t = -near_point.y / ray_direction.y;
        if t >= 0.0 {
            let intersection = near_point + ray_direction * t;
            return Some(intersection);
        }
    }

    return None;
}
