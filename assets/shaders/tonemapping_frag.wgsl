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
fn uncharted_tonemapping_func(
    A: f32, B: f32, C: f32, D: f32, E: f32, F: f32, x: vec3<f32>) -> vec3<f32>
{
    return ( (x * (A*x + C*B) + D*E) / (x * (A*x + B) + D*F) ) - E/F;
}
// ----------------------------------------------------------------------------
fn uncharted_2_tonemap(
    A: f32, B: f32, C: f32, D: f32, E: f32, F: f32, color: vec3<f32>
) -> vec3<f32>
{
    var mapped_linear_color = uncharted_tonemapping_func(A, B, C, D, E, F, color);
    mapped_linear_color = max(mapped_linear_color, vec3<f32>(0.0));

    var mapped_linear_white = uncharted_tonemapping_func(A, B, C, D, E, F, vec3<f32>(11.2));
    mapped_linear_white = max(mapped_linear_white, vec3<f32>(0.0));

    return mapped_linear_color / mapped_linear_white;
}
// ----------------------------------------------------------------------------
struct FragmentInput {
    [[builtin(position)]]                           frag_coord: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    texture_coord: vec2<f32>;
    [[location(1), interpolate(flat)]]              exposure: f32;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var hdr_texture: texture_2d<f32>;
// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var<uniform> settings: Tonemapping;
// ----------------------------------------------------------------------------
struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> FragmentOutput {
    var hdr_color = textureLoad(hdr_texture, vec2<i32>(in.texture_coord), 0).rgb;

    # ifdef DISABLE_TONEMAPPING

    return FragmentOutput(vec4<f32>(hdr_color, 1.0));

    # else

    var final_color = uncharted_2_tonemap(
        settings.shoulder_strength,
        settings.linear_strength,
        settings.linear_angle,
        settings.toe_strength,
        settings.toe_numerator,
        settings.toe_denumerator,
        hdr_color * in.exposure,
    ) * settings.post_scale;

    return FragmentOutput(vec4<f32>(final_color, 1.0));

    # endif
}
// ----------------------------------------------------------------------------
