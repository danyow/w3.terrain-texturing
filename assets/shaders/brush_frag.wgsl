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
struct FragmentInput {
    [[builtin(position)]]                           position: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    coord_2d: vec2<f32>;
    [[location(1)]]                                 center: vec3<f32>;
    [[location(2)]]                                 brush_ring_width: f32;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var world_pos_texture: texture_2d<f32>;
// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var<uniform> brush: BrushInfo;
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    let world_pos = textureLoad(world_pos_texture, vec2<i32>(i32(in.coord_2d.x), i32(in.coord_2d.y)), 0);

    // distance on the plane
    let dist = distance(world_pos.xz, in.center.xz);

    if (dist >= brush.radius && dist <= (brush.radius + in.brush_ring_width)) {
        return vec4<f32>(brush.color.rgb, 0.75);

    } else {
        return vec4<f32>(1.0, 1.0, 1.0, 0.0);
    }
}
// ----------------------------------------------------------------------------
