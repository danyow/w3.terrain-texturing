struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    lod: u32;
};

[[group(1), binding(0)]] var<uniform> mesh: Mesh;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
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

    fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    return fragmentCol;
}
