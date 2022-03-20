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

// lights
struct DirectionalLight {
    color: vec3<f32>;
    brightness: f32;
    direction: vec3<f32>;
};

// mesh
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    clipmap_and_lod: u32;
};

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

// view
[[group(0), binding(0)]] var<uniform> view: View;
[[group(0), binding(1)]] var<uniform> sunlight: DirectionalLight;

[[group(1), binding(0)]] var<uniform> mesh: Mesh;

// textures
[[group(2), binding(0)]] var textureArray: texture_2d_array<f32>;
[[group(2), binding(1)]] var terrainTextureSampler: sampler;
[[group(2), binding(2)]] var normalArray: texture_2d_array<f32>;
[[group(2), binding(3)]] var terrainNormalSampler: sampler;
[[group(2), binding(4)]] var<uniform> textureParams: TextureParameters;

// texturing
[[group(3), binding(0)]] var controlMap: texture_storage_2d_array<r16uint, read>;
[[group(3), binding(1)]] var tintmapArray: texture_2d_array<f32>;
[[group(3), binding(2)]] var tintmapSampler: sampler;
[[group(3), binding(3)]] var<uniform> clipmap: ClipmapInfo;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    let fragmentPos = in.world_position.xyz;

    let gamma = 2.2;

    // https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
    let barys = vec3<f32>(in.uv.x, in.uv.y, 1.0 - in.uv.x - in.uv.y);
    let minBarys = min(barys.x, min(barys.y, barys.z));
    // fwidth = abs(dpdx(minBarys)) + abs(dpdy(minBarys));
    let delta = fwidth(minBarys);

    // color mesh depending on current lod level
    let lod = mesh.clipmap_and_lod >> 16u;
    let clipmap_level = mesh.clipmap_and_lod & 15u;

    let r = lod % 2u;
    let g = r + lod % 4u;
    let b = r + lod % 3u;
    let wireframeCol = 0.2 * vec4<f32>((1.0 + f32(r)) / 2.0, f32(g) / 3.0, f32(b) / 2.0, 0.0);
    let clipmapCol = vec4<f32>(1.0);

    let wireframeWidth = 0.75 * delta;

    // test texturing
    // xyz -> xyz
    let fragmentPosFlipped = in.world_position.xzy;

    // test clipmap
    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset);
    let mapScaling: f32 = clipmap.layers[clipmap_level].resolution;
    let mapSize: f32 = clipmap.layers[clipmap_level].size;

    var controlMapPos: vec2<f32> = (fragmentPosFlipped.xy - clipmap.world_offset) / clipmap.world_res;
    controlMapPos = (controlMapPos - mapOffset) / mapScaling;

    let controlMapPosCoord: vec2<i32> =  clamp(vec2<i32>(controlMapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));
    let controlMapValueA: vec4<u32> = textureLoad(controlMap, controlMapPosCoord, i32(clipmap_level));

    // scale texture
    let texturingPos = vec2<f32>(0.333) * fragmentPosFlipped.xy;

    let partialDDX: vec2<f32> = dpdx(fragmentPosFlipped.xy);
    let partialDDY: vec2<f32> = dpdy(fragmentPosFlipped.xy);
    // scale derivatives
    let scaledDDX = partialDDX * 0.333;
    let scaledDDY = partialDDY * 0.333;

    // test texture
    let overlayTextureA: u32 = (controlMapValueA.x & 31u) - 1u;
    let bkgrndTextureA: u32 = ((controlMapValueA.x >> 5u) & 31u) - 1u;

    var fragmentCol = textureSampleGrad(
        // textureArray, terrainTextureSampler, texturingPos, i32(bkgrndTextureA), scaledDDX, scaledDDY);
        textureArray, terrainTextureSampler, texturingPos, i32(overlayTextureA), scaledDDX, scaledDDY);

    fragmentCol = vec4<f32>(pow(fragmentCol.rgb, vec3<f32>(gamma)), 1.0);

    // --- lighting
    // phong-blinn
    let fragmentNormal = normalize(in.normal);
    // directional light
    // sun light coming from the sun
    let lightDirection = normalize(-sunlight.direction);

    // pointlight direction
    // let lightDirection = normalize(lightPos - fragmentPos);
    // let viewDirection = normalize(view.world_position.xyz - fragmentPos);
    let viewDirection = normalize(view.world_position.xyz);
    let halfwayDirection = normalize(lightDirection + viewDirection);

    let ambientStrength = 0.03;
    let diffuseStrength = max(dot(fragmentNormal, lightDirection), 0.0);
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

    fragmentCol = vec4<f32>(col * fragmentCol.rgb, 1.0);

    // debug visualization for wireframes and clipmap level
    // fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    // fragmentCol = mix(fragmentCol, clipmapCol, f32(clipmap_level) / 6.0);

    // --- gamma correction
    fragmentCol = pow(fragmentCol, vec4<f32>(1.0 / gamma));

    return fragmentCol;
}
