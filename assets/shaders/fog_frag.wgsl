// ----------------------------------------------------------------------------
// mostly based on info from:
//  https://astralcode.blogspot.com/2019/06/reverse-engineering-rendering-of.html
// ----------------------------------------------------------------------------
struct View {
    view_proj: mat4x4<f32>;
    view: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};
// ----------------------------------------------------------------------------
// lights
struct DirectionalLight {
    color: vec3<f32>;
    direction: vec3<f32>;
};
// ----------------------------------------------------------------------------
// fog settings
struct FogColor {
    front: vec3<f32>;
    middle: vec3<f32>;
    back: vec3<f32>;
    final_exp: f32;
};
// ----------------------------------------------------------------------------
struct CustomizedSettings {
    color: vec3<f32>;
    color_scale: f32;
    color_bias: f32;
    amount: f32;
    amount_scale: f32;
    amount_bias: f32;
};
// ----------------------------------------------------------------------------
struct VerticalDensity {
    offset: f32;
    front: f32;
    back: f32;
    rim_range: f32;
};
// ----------------------------------------------------------------------------
struct EnvironmentFog {
    appear_distance: f32;
    appear_scale: f32;
    distance_clamp: f32;
    density: f32;
    vertical_density: VerticalDensity;
    color: FogColor;
    aerial_color: FogColor;
    custom: CustomizedSettings;
};
// ----------------------------------------------------------------------------
struct FragmentInput {
    [[builtin(position)]]                           position: vec4<f32>;
    [[location(0), interpolate(linear, center)]]    coord_2d: vec2<f32>;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var hdr_texture: texture_2d<f32>;
[[group(0), binding(1)]] var world_pos_texture: texture_2d<f32>;

// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var<uniform> view: View;
[[group(1), binding(1)]] var<uniform> mainlight: DirectionalLight;
[[group(1), binding(2)]] var<uniform> fog: EnvironmentFog;
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    let texture_pos = vec2<i32>(i32(in.coord_2d.x), i32(in.coord_2d.y));

    let hdr_col = textureLoad(hdr_texture, texture_pos, 0);
    let world_pos = textureLoad(world_pos_texture, texture_pos, 0);


    let camToFragVector = world_pos.xyz - view.world_position.xyz;
    let distanceToFragment = length(camToFragVector.xyz);

    let rayToFragmentDirection = camToFragVector / distanceToFragment;
    let clampedFragDistance = clamp(distanceToFragment, 0.0, fog.distance_clamp);

    // angle between main light and rayDirection to fragment will be used to
    // interpolate between different fog settings (e.g. different front/back
    // densiy and front/middle/back color gradient)
    let fogMainLightAngle = dot(mainlight.direction.xyz, rayToFragmentDirection);

    // ------------------------------------------------------------------------
    // calculate fog amount by sampling 16 times along ray (view -> fragment)
    // ------------------------------------------------------------------------
    let densityDirectionalWeight = clamp((fogMainLightAngle + fog.vertical_density.rim_range) / (1.0 + fog.vertical_density.rim_range), 0.0, 1.0);
    let vertFogDirectionalDensity = mix(fog.vertical_density.back, fog.vertical_density.front, densityDirectionalWeight);

    var accumulatedFogAmount = 1.0;
    var fogAppearDistanceScale = 0.0;

    if (clampedFragDistance >= fog.appear_distance) {

        let fogDensityPerSampleStep = fog.density * clampedFragDistance / 16.0;
        let vertFogDensityPerSampleStep = vertFogDirectionalDensity * rayToFragmentDirection.y * clampedFragDistance / 16.0;

        let fogVertStartHeight = (rayToFragmentDirection.y + view.world_position.y) - fog.vertical_density.offset;
        let vertFogBaseDensity = fogVertStartHeight * vertFogDirectionalDensity;

        for (var step = 16u; step > 0u; step = step - 1u) {
            let vertFogAmount = max(0.0, vertFogBaseDensity + f32(step) * vertFogDensityPerSampleStep);

            accumulatedFogAmount = accumulatedFogAmount * (1.0 - clamp(fogDensityPerSampleStep / (1.0 + vertFogAmount), 0.0, 1.0));
        }
        accumulatedFogAmount = abs(1.0 - accumulatedFogAmount);
        fogAppearDistanceScale = clamp((clampedFragDistance - fog.appear_distance) * fog.appear_scale, 0.0, 1.0);
    }

    let finalAerialFogAmount = fogAppearDistanceScale * pow(accumulatedFogAmount, fog.aerial_color.final_exp);
    let fogAmount = fogAppearDistanceScale * pow(accumulatedFogAmount, fog.color.final_exp);

    // ------------------------------------------------------------------------
    // calculate final fog adn aerial fog colors:
    //  - blend front/middle/back colors depending on light direction into directional colors
    //  - apply custom color to directional fog color
    // ------------------------------------------------------------------------
    // let gradientScale = 1.0 / 500.0;
    // let gradientBias = -150.0 * gradientScale;
    // let distance = clampedFragDistance * gradientScale + gradientBias;
    let frontToBackGradient = clamp((clampedFragDistance - 150.0) / 500.0, 0.0, 1.0) * abs(fogMainLightAngle) * abs(fogMainLightAngle);

    var directionalFogColor: vec3<f32>;
    var directionalAerialColor: vec3<f32>;

    let isFrontLight = 0.0 > fogMainLightAngle;

    if (isFrontLight) {
        directionalFogColor = fog.color.front.xyz;
        directionalAerialColor = fog.aerial_color.front;
    } else {
        directionalFogColor = fog.color.back;
        directionalAerialColor = fog.aerial_color.back;
    }
    let fogColor = mix(fog.color.middle, directionalFogColor, frontToBackGradient);
    let finalAerialFogColor = mix(fog.aerial_color.middle, directionalAerialColor, frontToBackGradient);

    // ------------------------------------------------------------------------
    // apply custom settings (fog color + amount scale) to directional fog color
    // ------------------------------------------------------------------------
    let customFogColorWeight = clamp(fogAmount * fog.custom.color_scale + fog.custom.color_bias, 0.0, 1.0);
    let customFogAmountWeight = clamp(fogAmount * fog.custom.amount_scale + fog.custom.amount_bias, 0.0, 1.0);

    let customFogAmount = mix(1.0, fog.custom.amount, customFogAmountWeight);

    let finalFogColor = mix(fogColor, fog.custom.color, customFogColorWeight);
    let finalFogAmount = clamp(fogAmount * customFogAmount, 0.0, 1.0);

    // ------------------------------------------------------------------------
    // apply fog
    // ------------------------------------------------------------------------
    let luminanceWeight = dot(vec3<f32>(0.333, 0.555, 0.222), hdr_col.rgb);

    var color = hdr_col.rgb;
    color = mix(color, luminanceWeight * finalAerialFogColor.rgb, finalAerialFogAmount);
    color = mix(color.rgb, finalFogColor, finalFogAmount);

    return vec4<f32>(color, 1.0);
}
// ----------------------------------------------------------------------------
