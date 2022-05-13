// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var hdr_texture: texture_2d<f32>;
// ----------------------------------------------------------------------------
struct VertexOutput {
    [[builtin(position)]]                           position: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    texture_coords: vec2<f32>;
};
// ----------------------------------------------------------------------------
[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    let tex_dim = textureDimensions(hdr_texture);

    let uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));

    // map uv to proper texture coordinates
    let texture_coords = vec2<f32>(f32(tex_dim.x) * uv.x, f32(tex_dim.y) - f32(tex_dim.y) * uv.y);
    let position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);

    return VertexOutput(position, texture_coords);
}
// ----------------------------------------------------------------------------
