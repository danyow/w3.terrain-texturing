// ----------------------------------------------------------------------------
// mostly based on info from:
//  https://astralcode.blogspot.com/2017/09/reverse-engineering-rendering-of.html
// and
//  Uncharted 2: HDR Lighting by John Hable presentation slides
// ----------------------------------------------------------------------------
struct Tonemapping {
    luminance_min: f32;
    luminance_max: f32;
    luminance_limit_shape: f32;
    shoulder_strength: f32;
    linear_strength: f32;
    linear_angle: f32;
    toe_strength: f32;
    toe_numerator: f32;
    toe_denumerator: f32;
    exposure_scale: f32;
    post_scale: f32;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var texture: texture_2d<f32>;
// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var<uniform> settings: Tonemapping;
// ----------------------------------------------------------------------------
struct VertexOutput {
    [[builtin(position)]]                           position: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    texture_coords: vec2<f32>;
    [[location(1), interpolate(flat)]]              exposure: f32;
};
// ----------------------------------------------------------------------------
fn adjust_exposure(
    luminance: f32,
    luminance_min: f32,
    luminance_max: f32,
    exposure_scale: f32,
    luminance_limit_shape: f32
) -> f32 {
    var avg_luminance = clamp(luminance, luminance_min, luminance_max);
    avg_luminance = max(avg_luminance, 1e-4);

    var scaled_whitepoint: f32 = exposure_scale * 11.2;

    var luma: f32 = avg_luminance / scaled_whitepoint;
    luma = pow(luma, luminance_limit_shape) * scaled_whitepoint;

    return exposure_scale / luma;
}
// ----------------------------------------------------------------------------
[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    let position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);

    // map uv to proper texture coordinates
    let tex_dim = textureDimensions(texture);
    let texture_coords = vec2<f32>(f32(tex_dim.x) * uv.x, f32(tex_dim.y) - f32(tex_dim.y) * uv.y);

    //TODO extract from data via compute shader
    let avg_luminance = 0.0315;

    var exposure = adjust_exposure(
        avg_luminance,
        settings.luminance_min,
        settings.luminance_max,
        settings.exposure_scale,
        settings.luminance_limit_shape,
    );

    return VertexOutput(position, texture_coords, exposure);
}
// ----------------------------------------------------------------------------
