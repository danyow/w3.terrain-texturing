// ----------------------------------------------------------------------------
struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};
// ----------------------------------------------------------------------------
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    clipmap_and_lod: u32;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var<uniform> view: View;
[[group(1), binding(0)]] var<uniform> mesh: Mesh;
// ----------------------------------------------------------------------------
struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    # ifdef SHOW_WIREFRAME
    [[location(2)]] uv: vec2<f32>;
    # endif
};
// ----------------------------------------------------------------------------
struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;

    # ifdef FLAT_SHADING
    [[location(1), interpolate(flat)]] normal: vec3<f32>;
    # else
    [[location(1)]] normal: vec3<f32>;
    # endif

    # ifdef SHOW_WIREFRAME
    [[location(2)]] uv: vec2<f32>;
    # endif
};
// ----------------------------------------------------------------------------
[[stage(vertex)]]
fn vertex(vertex: VertexInput) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.world_position = world_position;
    out.normal = vertex.normal;

    # ifdef SHOW_WIREFRAME
    out.uv = vertex.uv;
    # endif
    return out;
}
// ----------------------------------------------------------------------------
