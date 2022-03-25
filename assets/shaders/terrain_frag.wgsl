// ----------------------------------------------------------------------------
// view
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
// lights
struct DirectionalLight {
    color: vec3<f32>;
    brightness: f32;
    direction: vec3<f32>;
};
// ----------------------------------------------------------------------------
// mesh
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    clipmap_and_lod: u32;
};
// ----------------------------------------------------------------------------
// textures
struct TextureParam {
    blend_sharpness: f32;
    slope_base_dampening: f32;
    slope_normal_dampening: f32;
    specularity_scale: f32;
    specularity: f32;
    specularity_base: f32;
    specularity_scale_copy: f32;
    falloff: f32;
};

struct TextureParameters {
    param: array<TextureParam, 31u>;
};
// ----------------------------------------------------------------------------
// clipmap
struct ClipmapLayerInfo {
    map_offset: vec2<u32>;
    resolution: f32;
    size: f32;
};

struct ClipmapInfo {
    world_offset: vec2<f32>;
    world_res: f32;
    layers: array<ClipmapLayerInfo, 10u>;
};
// ----------------------------------------------------------------------------
// view
[[group(0), binding(0)]] var<uniform> view: View;
[[group(0), binding(1)]] var<uniform> sunlight: DirectionalLight;
// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var<uniform> mesh: Mesh;
// ----------------------------------------------------------------------------
// textures
[[group(2), binding(0)]] var textureArray: texture_2d_array<f32>;
[[group(2), binding(1)]] var terrainTextureSampler: sampler;
[[group(2), binding(2)]] var normalArray: texture_2d_array<f32>;
[[group(2), binding(3)]] var terrainNormalSampler: sampler;
[[group(2), binding(4)]] var<uniform> textureParams: TextureParameters;
// ----------------------------------------------------------------------------
// clipmap arrays for texture controlvalues (textureslots + uv scaling / blending) & tintmap
[[group(3), binding(0)]] var controlMap: texture_storage_2d_array<r16uint, read>;
[[group(3), binding(1)]] var tintmapArray: texture_2d_array<f32>;
[[group(3), binding(2)]] var tintmapSampler: sampler;
[[group(3), binding(3)]] var<uniform> clipmap: ClipmapInfo;
// ----------------------------------------------------------------------------
struct TextureMapping {
    diffuse: vec3<f32>;
    normal: vec3<f32>;
};
// ----------------------------------------------------------------------------
fn sample_texture(
    textureSlot: u32,
    texturingPos: vec2<f32>,
    scaleValue: f32,
    ddx: vec2<f32>,
    ddy: vec2<f32>,
) -> TextureMapping {
    var out: TextureMapping;

    // scale position and derivatives
    let texturingPos = texturingPos * scaleValue;
    let scaledDDX = ddx * scaleValue;
    let scaledDDY = ddy * scaleValue;

    out.diffuse = textureSampleGrad(
            textureArray, terrainTextureSampler, texturingPos, i32(textureSlot), scaledDDX, scaledDDY).rgb;

    out.normal = textureSampleGrad(
        normalArray, terrainNormalSampler, texturingPos, i32(textureSlot), scaledDDX, scaledDDY).xyz;

    // W3 uses dirextX normals -> invert green channel
    // TODO move to normalmap loading
    out.normal.g = 1.0 - out.normal.g;

    return out;
}
// ----------------------------------------------------------------------------
fn triplanar_mapping(
    triplanarWeights: vec3<f32>,
    worldPosition: vec3<f32>,
    worldNormal: vec3<f32>,

    textureSlot: u32,
    scaleValue: f32,

    partialDDX_xy: vec2<f32>,
    partialDDY_xy: vec2<f32>,

    partialDDX_xz: vec2<f32>,
    partialDDY_xz: vec2<f32>,

    partialDDX_yz: vec2<f32>,
    partialDDY_yz: vec2<f32>,

) -> TextureMapping {
    var output: TextureMapping;

    // initialize to zero
    output.diffuse = vec3<f32>(0.0);
    output.normal = vec3<f32>(0.0);

    // from presentation: prefer branching than sampling
    if (triplanarWeights.z > 0.0) {
        var texturingPos = vec2<f32>(worldPosition.x, -worldPosition.y);

        if (worldNormal.z < 0.0) {
            texturingPos.y = -texturingPos.y;
        }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xy, partialDDY_xy);
        output.diffuse = triplanarWeights.z * a.diffuse;
        output.normal = triplanarWeights.z * a.normal;
    }

    if (triplanarWeights.y > 0.0) {
        let texturingPos = worldPosition.xz;

        // terrain normal never points down
        // if (worldNormal.y < 0.0) {
        //     texturingPos.x = -texturingPos.x;
        // }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xz, partialDDY_xz);
        output.diffuse = output.diffuse + triplanarWeights.y * a.diffuse;
        output.normal = output.normal + triplanarWeights.y * a.normal;
    }

    if (triplanarWeights.x > 0.0) {
        var texturingPos = vec2<f32>(-worldPosition.y, worldPosition.z);

        if (worldNormal.x < 0.0) {
            texturingPos.x = -texturingPos.x;
        }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_yz, partialDDY_yz);
        output.diffuse = output.diffuse + triplanarWeights.x * a.diffuse;
        output.normal = output.normal + triplanarWeights.x * a.normal;
    }

    return output;
}
// ----------------------------------------------------------------------------
fn sample_background_texture(
    weights: vec4<f32>,
    fragmentPos: vec3<f32>,
    fragmentNormal: vec3<f32>,
    textureSlots: vec4<u32>,
    scaleValues: vec4<u32>,

    partialDDX_xy: vec2<f32>,
    partialDDY_xy: vec2<f32>,
    partialDDX_xz: vec2<f32>,
    partialDDY_xz: vec2<f32>,
    partialDDX_yz: vec2<f32>,
    partialDDY_yz: vec2<f32>,
) -> TextureMapping {
    var scaleValueMapping = array<f32,8u>(0.33, 0.166, 0.05, 0.025, 0.0125, 0.0075, 0.00375, 0.0);

    var bkgrnd: TextureMapping;

    // const from presentation
    let tighten = vec3<f32>(0.576);
    // must be >= 0.0 to normalize properly
    var triplanarWeights: vec3<f32> = max(vec3<f32>(0.00), abs(fragmentNormal) - tighten);

    // normalize weights
    triplanarWeights = triplanarWeights / (triplanarWeights.x + triplanarWeights.y + triplanarWeights.z);

    // combine 4 samples
    let bkgrndA = triplanar_mapping(
        triplanarWeights,
        fragmentPos,
        fragmentNormal,
        textureSlots.x,
        scaleValueMapping[scaleValues.x],
        partialDDX_xy, partialDDY_xy,
        partialDDX_xz, partialDDY_xz,
        partialDDX_yz, partialDDY_yz
    );
    let bkgrndB = triplanar_mapping(
        triplanarWeights,
        fragmentPos,
        fragmentNormal,
        textureSlots.y,
        scaleValueMapping[scaleValues.y],
        partialDDX_xy, partialDDY_xy,
        partialDDX_xz, partialDDY_xz,
        partialDDX_yz, partialDDY_yz
    );
    let bkgrndC = triplanar_mapping(
        triplanarWeights,
        fragmentPos,
        fragmentNormal,
        textureSlots.z,
        scaleValueMapping[scaleValues.z],
        partialDDX_xy, partialDDY_xy,
        partialDDX_xz, partialDDY_xz,
        partialDDX_yz, partialDDY_yz
    );
    let bkgrndD = triplanar_mapping(
        triplanarWeights,
        fragmentPos,
        fragmentNormal,
        textureSlots.w,
        scaleValueMapping[scaleValues.w],
        partialDDX_xy, partialDDY_xy,
        partialDDX_xz, partialDDY_xz,
        partialDDX_yz, partialDDY_yz
    );

    bkgrnd.diffuse =
          weights.x * bkgrndA.diffuse + weights.y * bkgrndB.diffuse
        + weights.z * bkgrndC.diffuse + weights.w * bkgrndD.diffuse;

    bkgrnd.normal =
          weights.x * bkgrndA.normal + weights.y * bkgrndB.normal
        + weights.z * bkgrndC.normal + weights.w * bkgrndD.normal;

    return bkgrnd;
}
// ----------------------------------------------------------------------------
fn sample_overlay_texture(
    weights: vec4<f32>,
    fragmentPos: vec2<f32>,
    textureSlots: vec4<u32>,
    partialDDX: vec2<f32>,
    partialDDY: vec2<f32>,
) -> TextureMapping {
    var overlay: TextureMapping;

    let baseScale = 0.333;

    let overlayA = sample_texture(textureSlots.x, fragmentPos, baseScale, partialDDX, partialDDY);
    let overlayB = sample_texture(textureSlots.y, fragmentPos, baseScale, partialDDX, partialDDY);
    let overlayC = sample_texture(textureSlots.z, fragmentPos, baseScale, partialDDX, partialDDY);
    let overlayD = sample_texture(textureSlots.w, fragmentPos, baseScale, partialDDX, partialDDY);

    overlay.diffuse =
          weights.x * overlayA.diffuse + weights.y * overlayB.diffuse
        + weights.z * overlayC.diffuse + weights.w * overlayD.diffuse;

    overlay.normal =
          weights.x * overlayA.normal + weights.y * overlayB.normal
        + weights.z * overlayC.normal + weights.w * overlayD.normal;

    return overlay;
}
// ----------------------------------------------------------------------------
struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {

    let gamma = 2.2;

    // color mesh depending on current lod level
    let lod = mesh.clipmap_and_lod >> 16u;
    let clipmap_level = mesh.clipmap_and_lod & 15u;

    let fragmentPos = in.world_position.xyz;
    let fragmentNormal = normalize(in.normal.xyz);

    // --- clipmap position calculation
    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset);
    let mapScaling: f32 = clipmap.layers[clipmap_level].resolution;
    let mapSize: f32 = clipmap.layers[clipmap_level].size;

    var clipmapPos: vec2<f32> = (fragmentPos.xz - clipmap.world_offset) / clipmap.world_res;
    clipmapPos = (clipmapPos - mapOffset) / mapScaling;

    let clipmapPosCoord: vec2<i32> =  clamp(vec2<i32>(clipmapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));

    //  clipmap weighting of neighboring pixels
    let clipmapPosFrac: vec2<f32> = fract(clipmapPos);

    let fractionalWeights = vec4<f32>(
        (1.0 - clipmapPosFrac.x) * (1.0 - clipmapPosFrac.y),
        clipmapPosFrac.x * (1.0 - clipmapPosFrac.y),
        clipmapPosFrac.y * (1.0 - clipmapPosFrac.x),
        clipmapPosFrac.x * clipmapPosFrac.y,
    );

    // ------------------------------------------------------------------------
    // extract encoded info about textures and parameters to use
    //
    // 0..4 overlay texture idx
    // 5..9 background textures idx
    // 10..16 blend control
    //   10..12 UV scale
    //   13..15 slope threshold
    //
    let controlMapValueA: vec4<u32> = textureLoad(controlMap, clipmapPosCoord, i32(clipmap_level));
    let controlMapValueB: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(1, 0), i32(clipmap_level));
    let controlMapValueC: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(0, 1), i32(clipmap_level));
    let controlMapValueD: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(1, 1), i32(clipmap_level));

    // --- overlay textures
    let overlayTextureSlots = vec4<u32>(
        (controlMapValueA.x & 31u) - 1u,
        (controlMapValueB.x & 31u) - 1u,
        (controlMapValueC.x & 31u) - 1u,
        (controlMapValueD.x & 31u) - 1u
    );

    // --- bkgrnd textures
    let bkgrndTextureSlots = vec4<u32>(
        ((controlMapValueA.x >> 5u) & 31u) - 1u,
        ((controlMapValueB.x >> 5u) & 31u) - 1u,
        ((controlMapValueC.x >> 5u) & 31u) - 1u,
        ((controlMapValueD.x >> 5u) & 31u) - 1u
    );

    // --- uv scaling (only background texture)
    let slopeThresholds = vec4<u32>(
        (controlMapValueA.x >> 10u) & 3u,
        (controlMapValueB.x >> 10u) & 3u,
        (controlMapValueC.x >> 10u) & 3u,
        (controlMapValueD.x >> 10u) & 3u
    );

    // --- slope threshold
    let bkgrndUvScaling = vec4<u32>(
        (controlMapValueA.x >> 13u) & 3u,
        (controlMapValueB.x >> 13u) & 3u,
        (controlMapValueC.x >> 13u) & 3u,
        (controlMapValueD.x >> 13u) & 3u
    );
    // ------------------------------------------------------------------------
    // partial derivatives for triplanar mapping (background texture)
    let partialDDX_xy: vec2<f32> = dpdx(fragmentPos.xy);
    let partialDDY_xy: vec2<f32> = dpdy(fragmentPos.xy);

    let partialDDX_xz: vec2<f32> = dpdx(fragmentPos.xz);
    let partialDDY_xz: vec2<f32> = dpdy(fragmentPos.xz);

    let partialDDX_yz: vec2<f32> = dpdx(fragmentPos.yz);
    let partialDDY_yz: vec2<f32> = dpdy(fragmentPos.yz);
    // ------------------------------------------------------------------------
    // triplanar mapping of background texture
    // ------------------------------------------------------------------------
    let bkgrnd = sample_background_texture(
        fractionalWeights,
        fragmentPos,
        fragmentNormal,
        bkgrndTextureSlots,
        bkgrndUvScaling,
        partialDDX_xy,
        partialDDY_xy,
        partialDDX_xz,
        partialDDY_xz,
        partialDDX_yz,
        partialDDY_yz
    );
    // ------------------------------------------------------------------------
    // overlay texture (no scaling)
    // ------------------------------------------------------------------------
    let overlay = sample_overlay_texture(
        fractionalWeights,
        fragmentPos.xz,
        overlayTextureSlots,
        partialDDX_xy,
        partialDDY_xy
    );
    // ------------------------------------------------------------------------
    // TBN matrix for normals
    // Note: because terrain is generated from heightmap a base tangent vector is
    // assumed to be (1, 0, 0) (in the heightmap plane)
    // renorthogonalize tangent with respect to normal
    let fragmentTangent: vec3<f32> = normalize(vec3<f32>(1.0, 0.0, 0.0) - fragmentNormal * dot(vec3<f32>(1.0, 0.0, 0.0), fragmentNormal));
    let biTangent: vec3<f32> = cross(fragmentNormal, fragmentTangent);
    let TBN = mat3x3<f32>(fragmentTangent, biTangent, fragmentNormal);

    // transform normalmap normals into world space
    // normal vectors range is [-1..1] and mapped in texture to [0..1], so remap:
    var overlayNormal = normalize(overlay.normal.rgb * 2.0 - 1.0);
    overlayNormal = TBN * overlayNormal;

    var bkgrndNormal = normalize(bkgrnd.normal.rgb * 2.0 - 1.0);
    bkgrndNormal = TBN * bkgrndNormal;

    // TODO proper blending based on slope. TEST 1:1 blending
    let blendValue = 0.5;
    var diffuse = mix(bkgrnd.diffuse.rgb, overlay.diffuse.rgb, blendValue);
    var normal = normalize(mix(bkgrndNormal, overlayNormal, blendValue));

    // --- lighting
    // phong-blinn

    // directional light
    // sun light coming from the sun
    let lightDirection = normalize(-sunlight.direction);

    // pointlight direction
    // let lightDirection = normalize(lightPos - fragmentPos);
    // let viewDirection = normalize(view.world_position.xyz - fragmentPos);
    let viewDirection = normalize(view.world_position.xyz);
    let halfwayDirection = normalize(lightDirection + viewDirection);

    let ambientStrength = 0.003;
    let diffuseStrength = max(dot(normal, lightDirection), 0.0);
    let specularStrength = 0.5;
    // shininess
    let specularExp = 32.0;
    // let reflectDirection = reflect(-lightDirection, fragmentNormal);
    // let specular = pow(max(dot(viewDirection, reflectDirection), 0.0), specularExp); // phong
    let specular = pow(max(dot(fragmentNormal, halfwayDirection), 0.0), 1.0 * specularExp);

    let ambientCol = sunlight.color * ambientStrength;
    let diffuseCol = diffuseStrength * sunlight.color;
    let specularCol = specularStrength * specular * sunlight.color;

    let col = ambientCol + diffuseCol + specularCol;

    var fragmentCol = vec4<f32>(col * diffuse.rgb, 1.0);

    // --------------------------------------------------------------------------------------------
    // debug visualization for wireframes and clipmap level

    // https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
    let barys = vec3<f32>(in.uv.x, in.uv.y, 1.0 - in.uv.x - in.uv.y);
    let minBarys = min(barys.x, min(barys.y, barys.z));
    // fwidth = abs(dpdx(minBarys)) + abs(dpdy(minBarys));
    let delta = fwidth(minBarys);

    let r = lod % 2u;
    let g = r + lod % 4u;
    let b = r + lod % 3u;
    let wireframeCol = 0.2 * vec4<f32>((1.0 + f32(r)) / 2.0, f32(g) / 3.0, f32(b) / 2.0, 0.0);
    let clipmapCol = vec4<f32>(1.0);

    let wireframeWidth = 0.75 * delta;

    // fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    // fragmentCol = mix(fragmentCol, clipmapCol, f32(clipmap_level) / 6.0);
    // --------------------------------------------------------------------------------------------

    // --- gamma correction
    // fragmentCol = pow(fragmentCol, vec4<f32>(1.0 / gamma));

    return fragmentCol;
}
// ----------------------------------------------------------------------------
