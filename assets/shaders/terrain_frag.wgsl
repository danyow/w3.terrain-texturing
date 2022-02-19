struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var fragmentCol = vec4<f32>(1.0, 0.0, 0.0, 1.0);

    // https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
    let barys = vec3<f32>(in.uv.x, in.uv.y, 1.0 - in.uv.x - in.uv.y);
    let minBarys = min(barys.x, min(barys.y, barys.z));
    // fwidth = abs(dpdx(minBarys)) + abs(dpdy(minBarys));
    let delta = fwidth(minBarys);

    let wireframeWidth = 0.75 * delta;
    let wireframeCol = vec4<f32>(0.5, 0.0, 0.0, 0.0) * 0.1;

    fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    return fragmentCol;
}
