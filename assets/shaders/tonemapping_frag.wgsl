// ----------------------------------------------------------------------------
struct FragmentInput {
    [[builtin(position)]]                           frag_coord: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    texture_coord: vec2<f32>;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var hdr_texture: texture_2d<f32>;
// ----------------------------------------------------------------------------
struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> FragmentOutput {
    var hdr_color = textureLoad(hdr_texture, vec2<i32>(in.texture_coord), 0).rgb;

    return FragmentOutput(vec4<f32>(hdr_color, 1.0));
}
// ----------------------------------------------------------------------------
