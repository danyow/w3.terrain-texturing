struct HeightmapParams {
    map_resolution: f32;
    map_height_scaling: f32;
    data_width: u32;
};

struct Heightmap {
    data: [[stride(4)]] array<u32>;
};

struct Normals {
    data: [[stride(4)]] array<f32>;
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
    let x_prev = clamp(invocation_id.x + 1u, 1u, params.data_width + 1u) - 1u;
    let x = invocation_id.x;
    let x_next = clamp(invocation_id.x + 1u, 0u, params.data_width);

    // prev and next line are provided in data so there are no seams between
    // slices
    let y_prev = invocation_id.y;
    let y = invocation_id.y + 1u;
    let y_next = invocation_id.y + 2u;

    // sample heightmap at a..i positions
    let ha = sample(x_prev, y_prev);
    let hb = sample(x, y_prev);
    let hc = sample(x_next, y_prev);

    let hd = sample(x_prev, y);
    let he = sample(x, y);
    let hf = sample(x_next, y);

    let hg = sample(x_prev, y_next);
    let hh = sample(x, y_next);
    let hi = sample(x_next, y_next);

    // interpolate m..p
    let hm = he + 0.25 * (abs(he - ha) + abs(hd - hb));
    let hn = hf + 0.25 * (abs(hf - hb) + abs(he - hc));
    let ho = hh + 0.25 * (abs(hh - hd) + abs(hg - he));
    let hp = hi + 0.25 * (abs(hi - he) + abs(hh - hf));

    // create vertices
    let fx = f32(x);
    let fy = f32(y);

    let scale = vec3<f32>(params.map_resolution, params.map_height_scaling, params.map_resolution);
    let vb = vec3<f32>(fx,       hb, fy - 1.0) * scale;
    let vd = vec3<f32>(fx - 1.0, hd, fy      ) * scale;
    let ve = vec3<f32>(fx,       he, fy      ) * scale;
    let vf = vec3<f32>(fx + 1.0, hf, fy      ) * scale;
    let vh = vec3<f32>(fx,       hh, fy + 1.0) * scale;

    let vm = vec3<f32>(fx - 0.5, hm, fy - 0.5) * scale;
    let vn = vec3<f32>(fx + 0.5, hn, fy - 0.5) * scale;
    let vo = vec3<f32>(fx - 0.5, ho, fy + 0.5) * scale;
    let vp = vec3<f32>(fx + 0.5, hp, fy + 0.5) * scale;

    let normal = normalize(
          normalize(cross(vh - ve, vp - ve))
        + normalize(cross(vp - ve, vf - ve))
        + normalize(cross(vf - ve, vn - ve))
        + normalize(cross(vn - ve, vb - ve))
        + normalize(cross(vb - ve, vm - ve))
        + normalize(cross(vm - ve, vd - ve))
        + normalize(cross(vd - ve, vo - ve))
        + normalize(cross(vo - ve, vh - ve))
    );

    // stride 3 for x, y, z normal components
    let target_location = (invocation_id.y * params.data_width + invocation_id.x) * 3u;
    normals.data[target_location] = normal.x;
    normals.data[target_location + 1u] = normal.y;
    normals.data[target_location + 2u] = normal.z;
}
