// ----------------------------------------------------------------------------
use bevy::prelude::Mesh;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
// ----------------------------------------------------------------------------
/// An XxZ-aligned Frame defined by its minimum and maximum point.
#[derive(Debug, Copy, Clone)]
pub struct XZGrid {
    size: f32,
    grid_spacing: f32,
}
// ----------------------------------------------------------------------------
impl XZGrid {
    pub fn new(size: f32, grid_spacing: f32) -> Self {
        Self {
            size,
            grid_spacing: grid_spacing.min(size).max(0.1),
        }
    }
}
// ----------------------------------------------------------------------------
impl From<XZGrid> for Mesh {
    // ------------------------------------------------------------------------
    fn from(sp: XZGrid) -> Self {
        let max_x = sp.size / 2.0;
        let min_x = -sp.size / 2.0;
        let max_z = sp.size / 2.0;
        let min_z = -sp.size / 2.0;

        let steps = (sp.size / sp.grid_spacing).floor() as usize;
        let vertex_count = 4 * steps;

        let normals = vec![[0.0, 1.0, 0.0]; vertex_count];
        let uvs = vec![[1.0, 0.0]; vertex_count];

        let mut vertices = vec![
            // frame
            [min_x, 0.0, min_z],
            [max_x, 0.0, min_z],
            [max_x, 0.0, max_z],
            [min_x, 0.0, max_z],
        ];
        for s in 1..steps {
            let s = s as f32;
            vertices.push([min_x + s * sp.grid_spacing, 0.0, min_z]);
            vertices.push([min_x + s * sp.grid_spacing, 0.0, max_z]);

            vertices.push([min_x, 0.0, min_z + s * sp.grid_spacing]);
            vertices.push([max_x, 0.0, min_z + s * sp.grid_spacing]);
        }

        let indices = if vertex_count < u16::MAX as usize {
            let mut indices = vec![
                // frame
                0, 1, 1, 2, 2, 3, 3, 0,
            ];
            for line in 1..steps {
                indices.push((line * 4) as u16);
                indices.push((line * 4) as u16 + 1);
                indices.push((line * 4) as u16 + 2);
                indices.push((line * 4) as u16 + 3);
            }
            Indices::U16(indices)
        } else {
            let mut indices = vec![
                // frame
                0, 1, 1, 2, 2, 3, 3, 0,
            ];
            for line in 1..steps {
                indices.push((line * 4) as u32);
                indices.push((line * 4) as u32 + 1);
                indices.push((line * 4) as u32 + 2);
                indices.push((line * 4) as u32 + 3);
            }
            Indices::U32(indices)
        };

        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
