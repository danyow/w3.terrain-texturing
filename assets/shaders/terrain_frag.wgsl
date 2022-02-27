struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    lod: u32;
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

[[group(1), binding(0)]] var<uniform> mesh: Mesh;

// textures
[[group(2), binding(0)]] var textureArray: texture_2d_array<f32>;
[[group(2), binding(1)]] var terrainTextureSampler: sampler;
[[group(2), binding(2)]] var normalArray: texture_2d_array<f32>;
[[group(2), binding(3)]] var terrainNormalSampler: sampler;
[[group(2), binding(4)]] var<uniform> textureParams: TextureParameters;

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
    let r = mesh.lod % 2u;
    let g = r + mesh.lod % 4u;
    let b = r + mesh.lod % 3u;
    var fragmentCol = vec4<f32>((1.0 + f32(r)) / 2.0, f32(g) / 3.0, f32(b) / 2.0, 0.0);

    let wireframeCol = fragmentCol * 0.2;
    let wireframeWidth = 0.75 * delta;

    // test texturing
    // xyz -> xyz
    let vertexPosFlipped = in.world_position.xzy;

    // scale texture
    let texturingPos = vec2<f32>(0.333) * vertexPosFlipped.xy;

    let partialDDX: vec2<f32> = dpdx(vertexPosFlipped.xy);
    let partialDDY: vec2<f32> = dpdy(vertexPosFlipped.xy);
    // scale derivatives
    let scaledDDX = partialDDX * 0.333;
    let scaledDDY = partialDDY * 0.333;

    // test texture
    let overlayTextureA: u32 = 0u;

    fragmentCol = textureSampleGrad(
        textureArray, terrainTextureSampler, texturingPos, i32(overlayTextureA), scaledDDX, scaledDDY);

    fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    return fragmentCol;
}
