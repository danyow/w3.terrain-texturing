// ----------------------------------------------------------------------------
// view
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
// map static info
struct TerrainMapInfo {
    // high u16 map size, low u16 clipmap level count
    size_and_clipmap_level_count: u32;
    resolution: f32;
    height_min: f32;
    height_max: f32;
    height_scaling: f32;
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
    size: f32;
    layers: array<ClipmapLayerInfo, 10u>;
};
// ----------------------------------------------------------------------------
struct TerrainShadowSettings {
    intensity: f32;
    falloff_smoothiness: f32;
    falloff_scale: f32;
    falloff_bias: f32;
    interpolation_distance: f32;
};
// ----------------------------------------------------------------------------
// view
[[group(0), binding(0)]] var<uniform> view: View;
[[group(0), binding(1)]] var<uniform> sunlight: DirectionalLight;
[[group(0), binding(2)]] var<uniform> mapInfo: TerrainMapInfo;
[[group(0), binding(3)]] var<uniform> shadows: TerrainShadowSettings;
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
[[group(3), binding(4)]] var heightmap: texture_storage_2d_array<r16uint, read>;
[[group(3), binding(5)]] var lightmap: texture_storage_2d_array<r16uint, read>;
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
        var texturingPos = vec2<f32>(-worldPosition.x, -worldPosition.y);

        if (worldNormal.z < 0.0) {
            texturingPos.y = 1.0 - texturingPos.y;
        }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xy, partialDDY_xy);
        output.diffuse = triplanarWeights.z * a.diffuse;
        output.normal = triplanarWeights.z * a.normal;
    }

    if (triplanarWeights.y > 0.0) {
        var texturingPos = worldPosition.xz;

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xz, partialDDY_xz);
        output.diffuse = output.diffuse + triplanarWeights.y * a.diffuse;
        output.normal = output.normal + triplanarWeights.y * a.normal;
    }

    if (triplanarWeights.x > 0.0) {
        var texturingPos = vec2<f32>(-worldPosition.z, worldPosition.y);

        if (worldNormal.x < 0.0) {
            texturingPos = 1.0-texturingPos;
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
fn compute_slope_blend(
    fragmentNormal: vec3<f32>,
    bkgrndNormal: vec3<f32>,
    baseDampening: f32,
    slopeThreshold: f32,
    blendSharpness: f32,
) -> f32 {

    // apply dampening to background normal
    let vertexSlope = dot(fragmentNormal, vec3<f32>(0.0, 1.0, 0.0));
    let flattenedBkgrndNormal = mix(bkgrndNormal, vec3<f32>(0.0, 1.0, 0.0), vertexSlope);
    let biasedBkgrndNormal = normalize(mix(bkgrndNormal, flattenedBkgrndNormal, baseDampening));

    // compute slope tangent
    // according to presentation (linear step between slopeThreshold and (slopeThreshold + blendSharpness)):
    // float verticalSurfaceTangent = ComputeSlopeTangent(
    //     biasedBkgrndNormal, slopeThreshold, saturate(slopeThreshold + bkgrndBlendSharpness) );
    //
    // rough approximation of slope (since it's a heightmap y will always be > 0)
    // TODO make it more accurate with trigonometric functions (tan(acos(y))) ?
    let slopeValue = (abs(biasedBkgrndNormal.x) + abs(biasedBkgrndNormal.z)) / biasedBkgrndNormal.y;

    // linear step (remap x \in [a..b] to parametric value t in [0..1] and clamp to [0, 1])
    let a = slopeThreshold;
    let b = slopeThreshold + blendSharpness;  // TODO clamp?
    let surfaceSlopeBlend = clamp((slopeValue - a) / (b - a), 0.0, 1.0);

    return surfaceSlopeBlend;
}
// ----------------------------------------------------------------------------
struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
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

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
    [[location(1)]] world_position: vec4<f32>;
};
// ----------------------------------------------------------------------------
[[stage(fragment)]]
fn fragment(in: FragmentInput) -> FragmentOutput {

    // color mesh depending on current lod level
    let lod = mesh.clipmap_and_lod >> 16u;
    var clipmap_level = mesh.clipmap_and_lod & 15u;

    let fragmentPos = in.world_position.xyz;
    let fragmentNormal = normalize(in.normal.xyz);

    // --- clipmap position calculation
    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset);
    let mapScaling = clipmap.layers[clipmap_level].resolution;
    let mapSize = clipmap.layers[clipmap_level].size;

    var clipmapPos = (fragmentPos.xz - clipmap.world_offset) / clipmap.world_res;
    clipmapPos = (clipmapPos - mapOffset) / mapScaling;

    var clipmapPosCoord = clamp(vec2<i32>(clipmapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));

    //  clipmap weighting of neighboring pixels
    let clipmapPosFrac = fract(clipmapPos);

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
    //   10..12 slope threshold
    //   13..15 UV scale
    //
    let controlMapValueA: vec4<u32> = textureLoad(controlMap, clipmapPosCoord, i32(clipmap_level));
    let controlMapValueB: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(1, 0), i32(clipmap_level));
    let controlMapValueC: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(0, 1), i32(clipmap_level));
    let controlMapValueD: vec4<u32> = textureLoad(controlMap, clipmapPosCoord + vec2<i32>(1, 1), i32(clipmap_level));

    // Note about zero texture id: zero slot represents a terrain hole
    // by subtracting 1 the id overflows and the shader uses the *last* texture
    // in the texture array. this is a dedicated placeholder texture in the editor
    // independent of the loaded materialset to visualize terrain holes
    // it seems only background zero texture indicates a terrain hole (overlay
    // texture is ignored in the case). unclear what a non-zero bkgrnd and zero
    // overlay texture represent.

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

    // --- slope threshold
    let slopeThreshold = vec4<u32>(
        (controlMapValueA.x >> 10u) & 7u,
        (controlMapValueB.x >> 10u) & 7u,
        (controlMapValueC.x >> 10u) & 7u,
        (controlMapValueD.x >> 10u) & 7u
    );

    // --- uv scaling (only background texture)
    let bkgrndUvScaling = vec4<u32>(
        (controlMapValueA.x >> 13u) & 7u,
        (controlMapValueB.x >> 13u) & 7u,
        (controlMapValueC.x >> 13u) & 7u,
        (controlMapValueD.x >> 13u) & 7u
    );
    // ------------------------------------------------------------------------
    // interpolate slope threshold from controlmap values
    // ------------------------------------------------------------------------
    var slopeThresholdMapping = array<f32,8u>(0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.98);

    let slopeThreshold =
          fractionalWeights.x * slopeThresholdMapping[slopeThreshold.x]
        + fractionalWeights.y * slopeThresholdMapping[slopeThreshold.y]
        + fractionalWeights.z * slopeThresholdMapping[slopeThreshold.z]
        + fractionalWeights.w * slopeThresholdMapping[slopeThreshold.w];
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
    // interpolate background texture material paramss based on neighboring controlmap textures
    // ------------------------------------------------------------------------
    let bkgrndTextureParamsA = textureParams.param[bkgrndTextureSlots.x];
    let bkgrndTextureParamsB = textureParams.param[bkgrndTextureSlots.y];
    let bkgrndTextureParamsC = textureParams.param[bkgrndTextureSlots.z];
    let bkgrndTextureParamsD = textureParams.param[bkgrndTextureSlots.w];

    let bkgrndBaseDampening =
          fractionalWeights.x * bkgrndTextureParamsA.slope_base_dampening
        + fractionalWeights.y * bkgrndTextureParamsB.slope_base_dampening
        + fractionalWeights.z * bkgrndTextureParamsC.slope_base_dampening
        + fractionalWeights.w * bkgrndTextureParamsD.slope_base_dampening;

    let bkgrndNormalDampening =
          fractionalWeights.x * bkgrndTextureParamsA.slope_normal_dampening
        + fractionalWeights.y * bkgrndTextureParamsB.slope_normal_dampening
        + fractionalWeights.z * bkgrndTextureParamsC.slope_normal_dampening
        + fractionalWeights.w * bkgrndTextureParamsD.slope_normal_dampening;

    // ------------------------------------------------------------------------
    // interpolate overlay texture material params based on neighboring controlmap textures
    // ------------------------------------------------------------------------
    let overlayTextureParamsA = textureParams.param[overlayTextureSlots.x];
    let overlayTextureParamsB = textureParams.param[overlayTextureSlots.y];
    let overlayTextureParamsC = textureParams.param[overlayTextureSlots.z];
    let overlayTextureParamsD = textureParams.param[overlayTextureSlots.w];

    let overlayBlendSharpness =
          fractionalWeights.x * overlayTextureParamsA.blend_sharpness
        + fractionalWeights.y * overlayTextureParamsB.blend_sharpness
        + fractionalWeights.z * overlayTextureParamsC.blend_sharpness
        + fractionalWeights.w * overlayTextureParamsD.blend_sharpness;

    // ------------------------------------------------------------------------
    // combine normals based on material dampening parameter
    // ------------------------------------------------------------------------
    // transform normalmap normals into world space
    // normal vectors range is [-1..1] and mapped in texture to [0..1], so remap:
    var overlayNormal = normalize(overlay.normal.rgb * 2.0 - 1.0);
    var bkgrndNormal = normalize(bkgrnd.normal.rgb * 2.0 - 1.0);

    // Note: according to presentation the derivates are based on the world
    // normals. but combining them before the change it seems to look better in
    // some cases. though, this may just be covering some other incorrect
    // assumption/issue (e.g. normals from triplanar mapping?)
    //
    // normalCombination = CombineNormalsDerivates(
    //          bkgrndNormal, overlayNormal, float3(1.0 - bkgrndNormalDampening, bkgrndNormalDampening, 1.0)

    let overlayNormalDerivative: vec2<f32> = overlayNormal.xy / overlayNormal.z;
    let bkgrndNormalDerivative: vec2<f32> = bkgrndNormal.xy / bkgrndNormal.z;
    // mix based on bkgrnd material normal dampening
    let dampenedOverlayNormal: vec2<f32> = mix(
        overlayNormalDerivative.xy, bkgrndNormalDerivative.xy, bkgrndNormalDampening);

    overlayNormal = normalize(vec3<f32>(dampenedOverlayNormal.x, dampenedOverlayNormal.y, 1.0));

    // ------------------------------------------------------------------------
    // TBN matrix for normals
    // Note: because terrain is generated from heightmap a base tangent vector is
    // assumed to be (1, 0, 0) (in the heightmap plane)
    // renorthogonalize tangent with respect to normal
    let fragmentTangent: vec3<f32> = normalize(vec3<f32>(1.0, 0.0, 0.0) - fragmentNormal * dot(vec3<f32>(1.0, 0.0, 0.0), fragmentNormal));
    let biTangent: vec3<f32> = cross(fragmentNormal, fragmentTangent);
    let TBN = mat3x3<f32>(fragmentTangent, biTangent, fragmentNormal);

    bkgrndNormal = TBN * bkgrndNormal.xyz;
    overlayNormal = TBN * overlayNormal.xyz;

    // ------------------------------------------------------------------------
    // blending between background and overlay texture based on terrain slope
    // and controlmap slope threshold
    // ------------------------------------------------------------------------
    // bgrnd texture params are used for dampening and overlay texture defines
    // blending sharpness (edges)
    let surfaceSlopeBlend = compute_slope_blend(
        fragmentNormal,
        bkgrndNormal,
        bkgrndBaseDampening,
        slopeThreshold,
        overlayBlendSharpness,
    );

    # ifdef HIDE_OVERLAY_TEXTURE
        let surfaceSlopeBlend = 1.0;
    # endif
    # ifdef HIDE_BKGRND_TEXTURE
        let surfaceSlopeBlend = 0.0;
    # endif

    // combine based on slope tangent
    var diffuse = mix(overlay.diffuse.rgb, bkgrnd.diffuse.rgb, surfaceSlopeBlend);
    var normal = normalize(mix(overlayNormal, bkgrndNormal, surfaceSlopeBlend));

    # ifdef HIDE_OVERLAY_TEXTURE
    # ifdef HIDE_BKGRND_TEXTURE
        diffuse = vec3<f32>(1.0, 1.0, 1.0);
        normal = fragmentNormal;
    # endif
    # endif

    // --------------------------------------------------------------------------------------------
    // apply tint from clipmap
    // --------------------------------------------------------------------------------------------
    let normalizedPos = clipmapPos / clipmap.size;

    # ifdef IGNORE_TINT_MAP
    let tintmapColor = vec4<f32>(0.5);
    # else
    let tintmapColor = textureSample(tintmapArray, tintmapSampler, normalizedPos, i32(clipmap_level));
    # endif

    let darkenedTint = 2.0 * tintmapColor.rgb * diffuse.rgb;
    let screenblendTint = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - tintmapColor.rgb) * (vec3<f32>(1.0) - diffuse.rgb);

    if (tintmapColor.r < 0.5) { diffuse.r = darkenedTint.r; } else { diffuse.r = screenblendTint.r; }
    if (tintmapColor.g < 0.5) { diffuse.g = darkenedTint.g; } else { diffuse.g = screenblendTint.g; }
    if (tintmapColor.b < 0.5) { diffuse.b = darkenedTint.b; } else { diffuse.b = screenblendTint.b; }
    // --------------------------------------------------------------------------------------------

    # ifdef FLAT_SHADING
    normal = fragmentNormal;
    # endif
    // --- lighting
    // blinn-phong

    // directional light
    // sun light coming from the sun
    let lightDirection = normalize(-sunlight.direction);

    // pointlight direction
    // let lightDirection = normalize(lightPos - fragmentPos);
    // let viewDirection = normalize(view.world_position.xyz - fragmentPos);
    let viewDirection = normalize(view.world_position.xyz);
    let halfwayDirection = normalize(lightDirection + viewDirection);

    let ambientStrength = 0.125;
    let diffuseStrength = max(dot(normal, lightDirection), 0.0);
    let specularStrength = 0.125;
    // shininess
    let specularExp = 32.0;
    // let reflectDirection = reflect(-lightDirection, fragmentNormal);
    // let specular = pow(max(dot(viewDirection, reflectDirection), 0.0), specularExp); // phong
    let specular = pow(max(dot(fragmentNormal, halfwayDirection), 0.0), 1.0 * specularExp);

    let ambientCol = sunlight.color * ambientStrength;
    let diffuseCol = diffuseStrength * sunlight.color;
    let specularCol = specularStrength * specular * sunlight.color;

    // if the sun is below horizon only use ambient lighting
    let alpha = dot(vec3<f32>(0.0, 1.0, 0.0), lightDirection);
    // some smoothing of intensity cutoff
    let intensity = clamp((alpha - 0.01) / 0.2, 0.0, 1.0);

    var col = ambientCol + (diffuseCol + specularCol) * intensity;

    # ifdef DISABLE_TERRAIN_SHADOWS

    var fragmentCol = vec4<f32>(col * diffuse.rgb, 1.0);

    # else
    // --------------------------------------------------------------------------------------------
    // Terrain shadows
    // --------------------------------------------------------------------------------------------
    // Quick Hack for clipmap mismatch (?)
    var hackedOffset = vec2<f32>(-1.0, -1.0);
    if (clipmap_level == 1u) {
        hackedOffset = vec2<f32>(-1.0, -2.0);
    }

    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset) + hackedOffset;
    var clipmapPos = (fragmentPos.xz - clipmap.world_offset) / clipmap.world_res;
    clipmapPos = (clipmapPos - mapOffset) / mapScaling;
    let clipmapPosCoord = clamp(vec2<i32>(clipmapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));

    let light_height1: vec4<u32> = textureLoad(lightmap, clipmapPosCoord, i32(clipmap_level));
    let height_texel: vec4<u32> = textureLoad(heightmap, clipmapPosCoord, i32(clipmap_level));

    // reduced light_height - height_error:
    //       mapInfo.height_min + f32(light_height.x) * height_scaling
    //    - (mapInfo.height_min + f32(height_texel.x) * height_scaling - fragmentPos.y)
    //
    // Note: use i32 to prevent overflow!
    var light_height = f32(i32(light_height1.x) - i32(height_texel.x)) * mapInfo.height_scaling + fragmentPos.y;

    # ifdef FAST_TERRAIN_SHADOWS

    let falloff = 1.0;
    let in_the_shadow = 1.0 - min(clamp(light_height - fragmentPos.y, 0.0, falloff) / falloff, shadows.intensity);
    var fragmentCol = vec4<f32>(col * diffuse.rgb * in_the_shadow, 1.0);

    # else // not FAST_TERRAIN_SHADOWS
    let fragment_distance = length(view.world_position.xyz - fragmentPos.xyz);

    // interpolate between neighboring samples based on fractional weights
    if (fragment_distance <= shadows.interpolation_distance) {
        let light_height2: vec4<u32> = textureLoad(lightmap, clipmapPosCoord + vec2<i32>(1, 0), i32(clipmap_level));
        let light_height3: vec4<u32> = textureLoad(lightmap, clipmapPosCoord + vec2<i32>(0, 1), i32(clipmap_level));
        let light_height4: vec4<u32> = textureLoad(lightmap, clipmapPosCoord + vec2<i32>(1, 1), i32(clipmap_level));

        let height_texel2: vec4<u32> = textureLoad(heightmap, clipmapPosCoord + vec2<i32>(1, 0), i32(clipmap_level));
        let height_texel3: vec4<u32> = textureLoad(heightmap, clipmapPosCoord + vec2<i32>(0, 1), i32(clipmap_level));
        let height_texel4: vec4<u32> = textureLoad(heightmap, clipmapPosCoord + vec2<i32>(1, 1), i32(clipmap_level));

        // reduced light_height - height_error:
        // Note: use i32 to prevent overflow!
        let light_height2 = f32(i32(light_height2.x) - i32(height_texel2.x)) * mapInfo.height_scaling + fragmentPos.y;
        let light_height3 = f32(i32(light_height3.x) - i32(height_texel3.x)) * mapInfo.height_scaling + fragmentPos.y;
        let light_height4 = f32(i32(light_height4.x) - i32(height_texel4.x)) * mapInfo.height_scaling + fragmentPos.y;

        light_height = fractionalWeights.x * light_height
            + fractionalWeights.y * light_height2
            + fractionalWeights.z * light_height3
            + fractionalWeights.w * light_height4;
    }

    // if the normal points along the light direction the fragment is pointing
    // away from the light (not a "direct" shadow "receiver" but a potential
    // shadow caster, e.g. backside of terrain). in this case the edge smoothiness
    // should be small(er). if it's oppositte direction the fragment is a
    // potential shadow receiver (aligment -> 1.0) and the edge smoothiness should
    // have a greater influence.
    // some base smootheness is used
    let light_alignment = 0.1 + clamp(dot(fragmentNormal, lightDirection), 0.0, 0.90);

    // parameters to control the amount of smoothing based on fragment distance:
    // shadow edges in the distance should be smoothed more (especially since very
    // long shadows have low/blocky resolution)
    let falloff = 0.1 + shadows.falloff_smoothiness
        * clamp(fragment_distance, shadows.falloff_bias, shadows.falloff_scale) / shadows.falloff_scale * light_alignment;

    // difference in calculated lightheight and terrain height implies that terrain
    // is either above lightray height (== no shadow) or below (== in the shadow)
    // falloff adds some room for height difference which will be normalized and
    // defines a how-much-in-the-shadow scale. the effect is a smoother shadow
    // edge (though it's also moving the shadow edge!). final intensity value
    // defines the base darkness of the shadow.
    let in_the_shadow = 1.0 - min(clamp(light_height - fragmentPos.y, 0.0, falloff) / falloff, shadows.intensity);

    var fragmentCol = vec4<f32>(col * diffuse.rgb * in_the_shadow, 1.0);
    # endif // FAST_TERRAIN_SHADOWS
    # endif // DISABLE_TERRAIN_SHADOWS
    // --------------------------------------------------------------------------------------------
    // debug visualization for texture control
    # ifdef SHOW_BLEND_VALUE
    fragmentCol = vec4<f32>(slopeThreshold, slopeThreshold, slopeThreshold, 1.0);
    # endif

    # ifdef SHOW_UV_SCALING
    let value = f32(bkgrndUvScaling.x) / 8.0;
    fragmentCol = vec4<f32>(value, value, value, 1.0);
    # endif
    // --------------------------------------------------------------------------------------------
    // debug visualization for normals
    # ifdef SHOW_FRAGMENT_NORMAL
    fragmentCol = vec4<f32>(fragmentNormal.xyz, 1.0);
    # endif

    # ifdef SHOW_COMBINED_NORMAL
    fragmentCol = vec4<f32>(normal.xyz, 1.0);
    # endif
    // --------------------------------------------------------------------------------------------
    // debug visualization for *direct* tint map values (not applied as above!)
    # ifdef SHOW_TINT_MAP
    fragmentCol = vec4<f32>(tintmapColor.rgb, 1.0);
    # endif
    // --------------------------------------------------------------------------------------------
    // debug visualization for *direct* tint map values (not applied as above!)
    # ifdef SHOW_LIGHTHEIGHT_MAP
    let light_height1: vec4<u32> = textureLoad(lightmap, clipmapPosCoord, i32(clipmap_level));
    fragmentCol = vec4<f32>(f32(light_height1.x) / 65535.0);
    # endif
    // --------------------------------------------------------------------------------------------
    // debug visualization for wireframes
    # ifdef SHOW_WIREFRAME
    // https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
    let barys = vec3<f32>(in.uv.x, in.uv.y, 1.0 - in.uv.x - in.uv.y);
    let minBarys = min(barys.x, min(barys.y, barys.z));
    // fwidth = abs(dpdx(minBarys)) + abs(dpdy(minBarys));
    let delta = fwidth(minBarys);

    let r = lod % 2u;
    let g = r + lod % 4u;
    let b = r + lod % 3u;
    let wireframeCol = 0.6 * vec4<f32>((1.0 + f32(r)) / 2.0, f32(g) / 3.0, f32(b) / 2.0, 0.0);

    let wireframeWidth = 0.75 * delta;

    fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    # endif
    // --------------------------------------------------------------------------------------------
    // debug visualization for clipmap level
    # ifdef SHOW_CLIPMAP_LEVEL
    let clipmapCol = vec4<f32>(1.0, 1.0, 0.0, 1.0);

    fragmentCol = mix(fragmentCol, clipmapCol, f32(clipmap_level) / 6.0);
    # endif
    // --------------------------------------------------------------------------------------------

    return FragmentOutput(fragmentCol, in.world_position);
}
// ----------------------------------------------------------------------------
