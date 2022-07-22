// ----------------------------------------------------------------------------
use bevy::prelude::Mesh;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
// ----------------------------------------------------------------------------
#[derive(Debug, Copy, Clone)]
pub struct CameraVisualization {
    len: f32,
}
// ----------------------------------------------------------------------------
impl CameraVisualization {
    // ------------------------------------------------------------------------
    pub fn new(len: f32) -> Self {
        Self { len }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<CameraVisualization> for Mesh {
    // ------------------------------------------------------------------------
    #[rustfmt::skip]
    fn from(cone: CameraVisualization) -> Self {
        let vertex_count = 5;

        let normals = vec![[0.0, 1.0, 0.0]; vertex_count];
        let uvs = vec![[1.0, 0.0]; vertex_count];

        let cone_width = cone.len * 0.1;
        let frame_distance = cone.len * 0.55;

        let vertices = vec![
            // base
            [0.0, 0.0, 0.0],
            // frame
            [2.0 *  cone_width,  cone_width, -frame_distance],
            [2.0 * -cone_width,  cone_width, -frame_distance],
            [2.0 * -cone_width, -cone_width, -frame_distance],
            [2.0 *  cone_width, -cone_width, -frame_distance],
        ];

        let indices = Indices::U16(vec![
            0, 1, 0, 2, 0, 3, 0, 4,
            1, 2, 2, 3, 3, 4, 4, 1
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for CameraVisualization {
    fn default() -> Self {
        Self { len: 10.0 }
    }
}
// ----------------------------------------------------------------------------
