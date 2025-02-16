use glam::{Vec3, Vec4};

pub const DU_DRAW_QUADS: i32 = 1;
pub const DU_DRAW_LINES: i32 = 2;
pub const DU_DRAW_POINTS: i32 = 3;
pub const RC_NULL_AREA: u8 = 0;
pub const RC_WALKABLE_AREA: u8 = 63;
pub const RC_MESH_NULL_IDX: u16 = 0xffff;

#[derive(Debug)]
pub struct PolyMesh {
    pub verts: Vec<Vec3>,     // Vertex positions
    pub polys: Vec<Vec<u16>>, // Polygons as vertex indices
    pub areas: Vec<u8>,       // Area ID for each polygon
    pub nvp: i32,             // Max vertices per polygon
    pub cs: f32,              // Cell size
    pub ch: f32,              // Cell height
    pub bmin: Vec3,           // Bounding box min
}

pub trait DebugDraw {
    fn begin(&mut self, prim: i32, size: f32);
    fn vertex(&mut self, pos: Vec3, color: Vec4);
    fn end(&mut self);
    fn area_to_col(&self, area: u8) -> Vec4;
}

pub fn du_debug_draw_poly_mesh(dd: &mut impl DebugDraw, mesh: &PolyMesh) {
    // Begin drawing triangles for polygon fills
    dd.begin(DU_DRAW_QUADS, 1.0);

    // Process each polygon
    for (i, poly) in mesh.polys.iter().enumerate() {
        let area = mesh.areas[i];

        // Determine polygon color based on area type
        let color = if area == RC_WALKABLE_AREA {
            Vec4::new(0.0, 0.75, 1.0, 0.25) // RGBA(0,192,255,64) -> walkable
        } else if area == RC_NULL_AREA {
            Vec4::new(0.0, 0.0, 0.0, 0.25) // RGBA(0,0,0,64) -> null area
        } else {
            dd.area_to_col(area) // Custom area colors
        };

        // Triangulate the polygon and draw
        for j in 2..mesh.nvp {
            let idx = j as usize;
            if idx >= poly.len() || poly[idx] == RC_MESH_NULL_IDX {
                break;
            }

            // Create a triangle fan from the polygon
            let vi = [poly[0], poly[idx - 1], poly[idx]];

            // Draw the triangle vertices
            for k in 0..3 {
                if vi[k] >= mesh.verts.len() as u16 {
                    continue;
                }
                let v = &mesh.verts[vi[k] as usize];
                let x = mesh.bmin.x + v.x * mesh.cs;
                let y = mesh.bmin.y + (v.y + 1.0) * mesh.ch;
                let z = mesh.bmin.z + v.z * mesh.cs;
                dd.vertex(Vec3::new(x, y, z), color);
            }
        }
    }
    dd.end();

    // Draw boundary edges
    dd.begin(DU_DRAW_LINES, 2.5);
    let col_boundary = Vec4::new(0.0, 0.25, 0.25, 0.86); // RGBA(0,48,64,220)

    for (i, poly) in mesh.polys.iter().enumerate() {
        for j in 0..mesh.nvp {
            let idx = j as usize;
            if idx >= poly.len() || poly[idx] == RC_MESH_NULL_IDX {
                break;
            }

            // Get indices for the edge vertices
            let nj = if j + 1 >= mesh.nvp.try_into().unwrap()
                || poly[(j + 1) as usize] == RC_MESH_NULL_IDX
            {
                0
            } else {
                j + 1
            };

            let vi = [poly[idx], poly[nj as usize]];

            // Draw edge line
            for k in 0..2 {
                if vi[k] >= mesh.verts.len() as u16 {
                    continue;
                }
                let v = &mesh.verts[vi[k] as usize];
                let x = mesh.bmin.x + v.x * mesh.cs;
                let y = mesh.bmin.y + (v.y + 1.0) * mesh.ch + 0.1;
                let z = mesh.bmin.z + v.z * mesh.cs;
                dd.vertex(Vec3::new(x, y, z), col_boundary);
            }
        }
    }
    dd.end();

    // Draw vertices as points
    dd.begin(DU_DRAW_POINTS, 3.0);
    let col_vertex = Vec4::new(0.0, 0.0, 0.0, 0.86); // RGBA(0,0,0,220)

    for vert in mesh.verts.iter() {
        let x = mesh.bmin.x + vert.x * mesh.cs;
        let y = mesh.bmin.y + (vert.y + 1.0) * mesh.ch + 0.1;
        let z = mesh.bmin.z + vert.z * mesh.cs;
        dd.vertex(Vec3::new(x, y, z), col_vertex);
    }
    dd.end();
}
