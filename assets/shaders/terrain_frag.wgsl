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
    [[location(1)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
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
    let vertexPosFlipped = in.world_position.xzy;

    // test clipmap
    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset);
    let mapScaling: f32 = clipmap.layers[clipmap_level].resolution;
    let mapSize: f32 = clipmap.layers[clipmap_level].size;

    var controlMapPos: vec2<f32> = (vertexPosFlipped.xy - clipmap.world_offset) / clipmap.world_res;
    controlMapPos = (controlMapPos - mapOffset) / mapScaling;

    let controlMapPosCoord: vec2<i32> =  clamp(vec2<i32>(controlMapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));
    let controlMapValueA: vec4<u32> = textureLoad(controlMap, controlMapPosCoord, i32(clipmap_level));

    // scale texture
    let texturingPos = vec2<f32>(0.333) * vertexPosFlipped.xy;

    let partialDDX: vec2<f32> = dpdx(vertexPosFlipped.xy);
    let partialDDY: vec2<f32> = dpdy(vertexPosFlipped.xy);
    // scale derivatives
    let scaledDDX = partialDDX * 0.333;
    let scaledDDY = partialDDY * 0.333;

    // test texture
    let overlayTextureA: u32 = (controlMapValueA.x & 31u) - 1u;
    let bkgrndTextureA: u32 = ((controlMapValueA.x >> 5u) & 31u) - 1u;

    var fragmentCol = textureSampleGrad(
        // textureArray, terrainTextureSampler, texturingPos, i32(bkgrndTextureA), scaledDDX, scaledDDY);
        textureArray, terrainTextureSampler, texturingPos, i32(overlayTextureA), scaledDDX, scaledDDY);

    // debug visualization for wireframes and clipmap level
    // fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    // fragmentCol = mix(fragmentCol, clipmapCol, f32(clipmap_level) / 6.0);

    return fragmentCol;
}
