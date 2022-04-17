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
    [[location(1)]] packed_normal: u32;
    # ifdef SHOW_WIREFRAME
    [[location(2)]] barycentric_flags: u32;
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
// unpack packed 11:10:11 normals to reduce memory consumption
//
// from kajiya renderer by Tomasz Stachowiak (h3r2tic), Embark Studios
// https://github.com/EmbarkStudios/kajiya/tree/main/assets/shaders/inc/mesh.hlsl#L23
fn unpack_unit_direction_11_10_11(packed: u32) -> vec3<f32> {
    return vec3<f32>(
        f32(packed & ((1u << 11u) - 1u)) * (2.0 / f32((1u << 11u) - 1u)) - 1.0,
        f32((packed >> 11u) & ((1u << 10u) - 1u)) * (2.0 / f32((1u << 10u) - 1u)) - 1.0,
        f32((packed >> 21u)) * (2.0 / f32((1u << 11u) - 1u)) - 1.0
    );
}
// ----------------------------------------------------------------------------
[[stage(vertex)]]
fn vertex(vertex: VertexInput) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.world_position = world_position;
    out.normal = unpack_unit_direction_11_10_11(vertex.packed_normal);

    # ifdef SHOW_WIREFRAME
    // decode barycentric flags into vec2
    out.uv = vec2<f32>(1.0 * f32(vertex.barycentric_flags & 1u), 0.5 * f32(vertex.barycentric_flags & 2u));
    # endif
    return out;
}
// ----------------------------------------------------------------------------
