struct HeightmapParams {
    map_resolution: f32;
    map_height_scaling: f32;
    data_width: u32;
};

struct Heightmap {
    data: [[stride(4)]] array<u32>;
};

struct Normals {
    data: [[stride(4)]] array<u32>;
};

[[group(0), binding(0)]]
var<uniform> params: HeightmapParams;

[[group(0), binding(1)]]
var<storage, read> heightmap: Heightmap;

[[group(0), binding(2)]]
var<storage, read_write> normals: Normals;

fn sample(x: u32, y: u32) -> f32 {
    let p = (y * params.data_width + x) / 2u;
    let v = heightmap.data[p];

    if (bool(x & 1u)) {
        return f32((v >> 16u));
    } else {
        return f32(v & 0xffffu);
    }
}
// ----------------------------------------------------------------------------
// pack normal vector as 11:10:11 to reduce memory consumption
//
// from kajiya renderer by Tomasz Stachowiak, Embark Studios
// https://github.com/EmbarkStudios/kajiya/tree/main/crates/lib/kajiya-asset/src/mesh.rs#L408
fn pack_unit_direction_11_10_11(x: f32, y: f32, z: f32) -> u32 {
    let x = u32((clamp(x, -1.0, 1.0) * 0.5 + 0.5) * f32((1u << 11u) - 1u));
    let y = u32((clamp(y, -1.0, 1.0) * 0.5 + 0.5) * f32((1u << 10u) - 1u));
    let z = u32((clamp(z, -1.0, 1.0) * 0.5 + 0.5) * f32((1u << 11u) - 1u));

    return (z << 21u) | (y << 11u) | x;
}
// ----------------------------------------------------------------------------
[[stage(compute), workgroup_size(8, 8, 1)]]
fn main(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    // a---b---c
    // |\ /|\ /|
    // | m | n |
    // |/ \|/ \|
    // d---e---f
    // |\ /|\ /|
    // | o | p |
    // |/ \|/ \|
    // g---h---i

    // clamp x to 0..datawidth for previous and next col
    // let x_prev = clamp(invocation_id.x + 0u, 1u, params.data_width + 1u) - 1u;
    let x = invocation_id.x;
    let x_next = clamp(invocation_id.x + 1u, 1u, params.data_width);

    // prev and next line are provided in data so there are no seams between
    // slices
    let y_prev = invocation_id.y;
    let y = invocation_id.y + 1u;
    let y_next = invocation_id.y + 2u;

    // sample heightmap at b, e, f, h positions
    let hb = sample(x, y_prev);
    let he = sample(x, y);
    let hf = sample(x_next, y);
    let hh = sample(x, y_next);

    let scale = vec3<f32>(params.map_resolution, params.map_height_scaling, params.map_resolution);
    let vb = vec3<f32>(0.0, hb, - 1.0) * scale;
    let ve = vec3<f32>(0.0, he,   0.0) * scale;
    let vf = vec3<f32>(1.0, hf,   0.0) * scale;
    let vh = vec3<f32>(0.0, hh,   1.0) * scale;

    let normal = normalize(
          normalize(cross(vh - ve, vf - ve))
        + normalize(cross(vf - ve, vb - ve))
    );

    let target_location = (invocation_id.y * params.data_width + x);

    normals.data[target_location] = pack_unit_direction_11_10_11(normal.x, normal.y, normal.z);
}
