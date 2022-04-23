// ----------------------------------------------------------------------------
struct BrushInfo {
    cam_pos: vec3<f32>;
    radius: f32;
    pos: vec2<f32>;
    ring_width: f32;
    max_visibility: f32;
    color: vec4<f32>;
    button: u32;
};
// ----------------------------------------------------------------------------
struct BrushResult {
    data: [[stride(4)]] array<f32>;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var world_pos_texture: texture_2d<f32>;
[[group(1), binding(0)]] var<uniform> brush: BrushInfo;
// ----------------------------------------------------------------------------
# ifdef STORE_RESULT
[[group(2), binding(0)]] var<storage, read_write> result: BrushResult;
# endif
// ----------------------------------------------------------------------------
struct VertexOutput {
    [[builtin(position)]]  position: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    texture_coords: vec2<f32>;
    [[location(1)]]                                 center: vec3<f32>;
    [[location(2)]]                                 adjusted_ring_width: f32;
};
// ----------------------------------------------------------------------------
[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    let tex_dim = textureDimensions(world_pos_texture);
    let mouse_pos = vec2<i32>(i32(brush.pos.x), tex_dim.y - i32(brush.pos.y));

    var world_pos = textureLoad(world_pos_texture, mouse_pos, 0);

    // restrict drawing of brush to a max distance to ensure only full res tiles
    // are covered.
    // Note: out of terrain mesh alpha is zero
    if (world_pos.w > 0.0 && distance(brush.cam_pos, world_pos.xyz) < brush.max_visibility) {
        // adjust ring width based on distance to cam
        let scale = clamp(distance(brush.cam_pos, world_pos.xyz) / 100.0, 0.2, 1.0);
        let adjusted_ring_width = brush.ring_width * scale;

        let uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));

        // map uv to proper texture coordinates
        let texture_coords = vec2<f32>(f32(tex_dim.x) * uv.x, f32(tex_dim.y) - f32(tex_dim.y) * uv.y);
        let position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
        let center = world_pos.xyz;

        # ifdef STORE_RESULT
        result.data[0] = center.x;
        result.data[1] = center.z;
        result.data[2] = brush.radius;
        result.data[3] = f32(brush.button);
        # endif

        return VertexOutput(position, texture_coords, center, adjusted_ring_width);
    } else {
        let uv = vec2<f32>(0.0);
        let texture_coords = vec2<f32>(0.0);
        let position = vec4<f32>(-1.0);
        let center = vec3<f32>(0.0);

        return VertexOutput(position, texture_coords, center, 0.0);
    }
}
// ----------------------------------------------------------------------------
