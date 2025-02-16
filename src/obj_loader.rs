use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub struct ObjData {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
}

#[derive(Debug)]
pub enum ObjLoadError {
    IoError(io::Error),
    ParseError(String),
}

impl From<io::Error> for ObjLoadError {
    fn from(error: io::Error) -> Self {
        ObjLoadError::IoError(error)
    }
}

pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<ObjData, ObjLoadError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    // OBJ files are 1-indexed, so we'll push a dummy vertex at index 0
    vertices.push(Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });

    for line in reader.lines() {
        let line = line?;
        let mut tokens = line.split_whitespace();

        match tokens.next() {
            Some("v") => {
                // Parse vertex
                let x = tokens.next().and_then(|s| s.parse().ok()).ok_or_else(|| {
                    ObjLoadError::ParseError("Invalid vertex x coordinate".to_string())
                })?;
                let y = tokens.next().and_then(|s| s.parse().ok()).ok_or_else(|| {
                    ObjLoadError::ParseError("Invalid vertex y coordinate".to_string())
                })?;
                let z = tokens.next().and_then(|s| s.parse().ok()).ok_or_else(|| {
                    ObjLoadError::ParseError("Invalid vertex z coordinate".to_string())
                })?;

                vertices.push(Vec3 { x, y, z });
            }
            Some("f") => {
                // Parse face: collect vertex indices
                let indices: Result<Vec<usize>, _> = tokens
                    .map(|token| {
                        // Handle vertex/texture/normal format by taking first number
                        token
                            .split('/')
                            .next()
                            .and_then(|idx| idx.parse().ok())
                            .ok_or_else(|| {
                                ObjLoadError::ParseError(format!("Invalid face index: {}", token))
                            })
                    })
                    .collect();

                faces.push(indices?);
            }
            // Ignore other lines
            _ => continue,
        }
    }

    Ok(ObjData { vertices, faces })
}

// Example usage and testing
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_obj() {
        // Create a temporary OBJ file
        let obj_content = "\
v -21.847065 -2.492895 19.569759
v -15.847676 -2.492895 18.838863
v -21.847065 -0.895197 19.569759
v -15.847676 -0.895197 18.838863
v -21.585381 -2.492895 21.717730
v -15.585992 -2.492895 20.986834
f 1 2 3 4 5
f 1 5 6";

        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), obj_content).unwrap();

        let obj_data = load_obj(temp_file.path()).unwrap();

        // Check vertices (remember we added a dummy vertex at index 0)
        assert_eq!(obj_data.vertices.len(), 7); // 6 + 1 dummy
        assert_eq!(obj_data.faces.len(), 2);

        // Check first vertex
        let first_vertex = &obj_data.vertices[1]; // Index 1 due to dummy vertex
        assert!((first_vertex.x - -21.847065).abs() < 1e-6);
        assert!((first_vertex.y - -2.492895).abs() < 1e-6);
        assert!((first_vertex.z - 19.569759).abs() < 1e-6);

        // Check faces
        assert_eq!(obj_data.faces[0], vec![1, 2, 3, 4, 5]);
        assert_eq!(obj_data.faces[1], vec![1, 5, 6]);
    }
}

// Utility functions for working with the loaded data
impl ObjData {
    // Get total number of vertices (excluding dummy vertex)
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() - 1 // Subtract dummy vertex
    }

    // Get total number of faces
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    // Convert all faces to triangles (using simple fan triangulation)
    pub fn triangulate(&self) -> Vec<[usize; 3]> {
        let mut triangles = Vec::new();

        for face in &self.faces {
            if face.len() >= 3 {
                // Triangulate as a fan from the first vertex
                for i in 1..(face.len() - 1) {
                    triangles.push([face[0], face[i], face[i + 1]]);
                }
            }
        }

        triangles
    }

    // Get bounds of the model
    pub fn get_bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3 {
            x: f32::INFINITY,
            y: f32::INFINITY,
            z: f32::INFINITY,
        };
        let mut max = Vec3 {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            z: f32::NEG_INFINITY,
        };

        // Skip dummy vertex at index 0
        for vertex in self.vertices.iter().skip(1) {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);

            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }

        (min, max)
    }
}
