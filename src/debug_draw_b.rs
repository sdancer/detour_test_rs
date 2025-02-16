use glam::{Vec2};
use glam::{Vec3, Vec4};
use std::f32::consts::PI;

// Input mesh data structure
pub struct InputMesh {
    pub verts: Vec<Vec3>,
    pub tris: Vec<i32>,
    pub normals: Vec<Vec3>,
}

impl InputMesh {
    pub fn new() -> Self {
        Self {
            verts: Vec::new(),
            tris: Vec::new(),
            normals: Vec::new(),
        }
    }
}

// Debug draw for input triangle mesh with slope visualization
pub fn du_debug_draw_tri_mesh_slope(
    dd: &mut impl DebugDraw,
    mesh: &InputMesh,
    walkable_slope_angle: f32,
    tex_scale: f32,
) {
    if mesh.verts.is_empty() || mesh.tris.is_empty() || mesh.normals.is_empty() {
        return;
    }

    // Calculate walkable threshold from slope angle
    let walkable_thr = (walkable_slope_angle / 180.0 * PI).cos();

    dd.texture(true);

    dd.begin(DU_DRAW_TRIS, 1.0);

    let unwalkable = Vec4::new(0.75, 0.5, 0.0, 1.0); // Similar to duRGBA(192,128,0,255)

    // Process triangles
    for i in (0..mesh.tris.len()).step_by(3) {
        let norm = &mesh.normals[i];

        // Calculate color based on slope
        let a = ((2.0 + norm.x + norm.y) / 4.0 * 220.0) as u8;
        let base_col = Vec4::new(a as f32 / 255.0, a as f32 / 255.0, a as f32 / 255.0, 1.0);

        let color = if norm.y < walkable_thr {
            lerp_col(base_col, unwalkable, 64.0 / 255.0)
        } else {
            base_col
        };

        // Get triangle vertices
        let va = &mesh.verts[mesh.tris[i] as usize];
        let vb = &mesh.verts[mesh.tris[i + 1] as usize];
        let vc = &mesh.verts[mesh.tris[i + 2] as usize];

        // Calculate texture coordinates
        let mut ax = 0;
        let mut ay = 0;

        if norm.y.abs() > norm[ax].abs() {
            ax = 1;
        }
        if norm.z.abs() > norm[ax].abs() {
            ax = 2;
        }

        ax = (1 << ax) & 3; // +1 mod 3
        ay = (1 << ax) & 3; // +1 mod 3

        let uva = tex_coord(va, ax, ay, tex_scale);
        let uvb = tex_coord(vb, ax, ay, tex_scale);
        let uvc = tex_coord(vc, ax, ay, tex_scale);

        dd.vertex_uv(*va, color, uva);
        dd.vertex_uv(*vb, color, uvb);
        dd.vertex_uv(*vc, color, uvc);
    }

    dd.end();
    dd.texture(false);
}

fn tex_coord(v: &Vec3, ax: usize, ay: usize, scale: f32) -> Vec2 {
    Vec2::new(v[ax] * scale, v[ay] * scale)
}

fn lerp_col(a: Vec4, b: Vec4, t: f32) -> Vec4 {
    a.lerp(b, t)
}

// Add to DebugDraw trait
pub trait DebugDraw {
    fn begin(&mut self, prim: i32, size: f32);
    fn end(&mut self);
    fn vertex(&mut self, pos: Vec3, color: Vec4);
    fn vertex_uv(&mut self, pos: Vec3, color: Vec4, uv: Vec2);
    fn texture(&mut self, state: bool);
}

// Constants
pub const DU_DRAW_TRIS: i32 = 2;
