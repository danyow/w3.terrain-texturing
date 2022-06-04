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
// clipmap
struct ClipmapLayerInfo {
    map_offset: vec2<u32>;
    resolution: f32;
    data_size: f32;
};

struct ClipmapInfo {
    world_offset: vec2<f32>;
    world_res: f32;
    clipmap_size: f32;
    layers: array<ClipmapLayerInfo, 10u>;
};
// ----------------------------------------------------------------------------
// Contains transformed ClipmapLayerInfo offsets independent of ray direction:
// step_1 and step_after contain map offsets [0..map_size[ in the direction of
// the ray.
struct DirectionalClipmapLayerInfo {
    // first step within next res level
    step_1: u32;
    // first step *after* next res level
    step_after: u32;
    // first ray within next res level
    ray_1: u32;
    // first ray *after* next res level
    ray_after: u32;
};

struct DirectionalClipmapInfo {
    layer: array<DirectionalClipmapLayerInfo, 10u>;
};
// ----------------------------------------------------------------------------
struct SliceInfo {
    highest_res_clipmap_level: u32;
    step_after: u32;
    schedule_id: u32;
    padding: u32;
};
// ----------------------------------------------------------------------------
struct ComputeSlices {
    // TODO reduce SliceInfo to one u32 IF array alignment can be reduced to 4b
    slice: array<SliceInfo, 10u>;
    count_and_lowest_level: u32;
};
// ----------------------------------------------------------------------------
struct ThreadWork {
    rays: u32;
    start_ray: u32;
    clipmap_level: u32;
    padding: u32;
};

struct ThreadSchedule {
    // TODO reduce ThreadWork to one u32 IF array alignment can be reduced to 4b
    slice: array<array<ThreadWork, 1024>, 5u>;
};
// ----------------------------------------------------------------------------
struct LightraysInfo {
    // defines an additional offset for choosing two interpolation points:
    //  lightPosBuf[offset + ray - 1] and lightPosBuf[offset + ray]
    //
    // offset 0 implies: interpolation between [ray-1] and [ray]
    // offset 1 implies: interpolation between [ray] and [ray+1]
    //
    lightpos_offset: u32;
    interpolation_weight: f32;
    ray_height_delta_per_step: f32;
};
// ----------------------------------------------------------------------------
[[group(0), binding(0)]] var lightmap: texture_storage_2d_array<r16uint, read_write>;
[[group(0), binding(1)]] var<uniform> map: TerrainMapInfo;
// ----------------------------------------------------------------------------
[[group(1), binding(0)]] var heightmap: texture_storage_2d_array<r16uint, read>;
[[group(1), binding(1)]] var<uniform> clipmap: ClipmapInfo;
[[group(1), binding(2)]] var<uniform> rayClipmap: DirectionalClipmapInfo;
[[group(1), binding(3)]] var<uniform> computeSlices: ComputeSlices;
[[group(1), binding(4)]] var<storage, read> threadSchedule: ThreadSchedule;
// ----------------------------------------------------------------------------
[[group(2), binding(0)]] var<uniform> settings: LightraysInfo;
// ----------------------------------------------------------------------------
// Note: workgroup size is fixed implies clipmap size is also fixed!
// ----------------------------------------------------------------------------
// workgroup shared memory
// holds the currently calculated lightheight after every step for every ray and
// two border values (1024 rays + 2 cornerpoints) for every clipmap layer (up to 5).
// needs to be shared between between all invocations within workgroup as every
// ray may require the result of a neighboring lightheight for interpolation.
// buffersize is 2 * (1024 + 2) for every clipmap layer so it can be split into
// a "read-only" part for previous results and a "write-only" part for current
// results. on next step the offset will be switched between the buffer halves.
var<workgroup> lightPosBuf: array<array<f32, 2052>, 5u>;
// ----------------------------------------------------------------------------
fn readBuf(step: u32) -> u32 {
    return (step % 2u) * 1026u;
}
// ----------------------------------------------------------------------------
fn writeBuf(step: u32) -> u32 {
    return ((step + 1u) % 2u) * 1026u;
}
// ----------------------------------------------------------------------------
fn to_map_coord_horizontal(ray_number: u32, ray_step: u32) -> vec2<i32> {
    return vec2<i32>(i32(ray_step), i32(ray_number));
}
// ----------------------------------------------------------------------------
fn to_map_coord_vertical(ray_number: u32, ray_step: u32) -> vec2<i32> {
    return vec2<i32>(i32(ray_number), i32(ray_step));
}
// ----------------------------------------------------------------------------
fn extract_lightheight(
    clipmapLevel: u32,
    ray_number: u32,
    // step is used for "synchronized" switching between buffer (aka distinct read and write offsets into same buffer)
    sequence_step: u32,
    ray_step_height_delta: f32,
) -> f32 {
    let readOffset = readBuf(sequence_step) + ray_number + settings.lightpos_offset;

    // interpolation between two neighboring lightheights
    let last_light_pos1 = lightPosBuf[clipmapLevel][readOffset];
    let last_light_pos2 = lightPosBuf[clipmapLevel][readOffset + 1u];

    // mix(x, y, z) = x * (1 - z) + y * z
    let interpolated_last_light_pos_height = mix(last_light_pos2, last_light_pos1, settings.interpolation_weight);

    return interpolated_last_light_pos_height + ray_step_height_delta;
}
// ----------------------------------------------------------------------------
fn store_lightheight(
    clipmapLevel: u32,
    ray_number: u32,
    // step is used for "synchronized" switching between buffer (aka distionct read and write offsets into same buffer)
    sequence_step: u32,
    light_height: f32)
{
    // Note: first entry is additional borderrpoint therefore ray 0 is at offset 1
    lightPosBuf[clipmapLevel][writeBuf(sequence_step) + ray_number + 1u] = light_height;
}
// ----------------------------------------------------------------------------
// helper
struct LightHeightResult {
    lightHeight: f32;
    lightmapHeight: u32;
};
// ----------------------------------------------------------------------------
fn calculate_lightheight(
    clipmapLevel: u32,
    ray_number: u32,
    sequence_step: u32,
    ray_step_height_delta: f32,
    mapCoord: vec2<i32>,
) -> LightHeightResult {

    let current_light_height = extract_lightheight(
        clipmapLevel, ray_number, sequence_step, ray_step_height_delta,
    );

    let current_terrain_height = textureLoad(heightmap, mapCoord, i32(clipmapLevel));
    let world_height = f32(current_terrain_height.x) * map.height_scaling;

    var light_height: f32;
    var lightmap_height: u32;

    if (world_height >= current_light_height) {
        // light hits terrain
        // TODO maybe increase/decrease height slightly to have a controlable offset
        light_height = world_height;
        lightmap_height = current_terrain_height.x;
    } else {
        // terrain below light, keep light height for next iteration
        light_height = current_light_height;
        lightmap_height = u32(current_light_height / map.height_scaling);
    }
    store_lightheight(clipmapLevel, ray_number, sequence_step, light_height);

    return LightHeightResult(light_height, lightmap_height);
}
// ----------------------------------------------------------------------------
fn init_last_lightpos(ray_number: u32, clipmap_levels: u32) {
    for (var clipmapLevel = 0u; clipmapLevel < clipmap_levels; clipmapLevel = clipmapLevel + 1u) {

        let readOffset = 0u;
        let writeOffset = 1026u;
        let first_pos_read_buf = 0u;
        let first_pos_write_buf = 1026u;

        // first cornerpoints for read (0) and write (1026) buffer parts
        lightPosBuf[clipmapLevel][0u] = -map.height_max;
        lightPosBuf[clipmapLevel][1026u] = -map.height_max;

        // last cornerpoints for read (1026-1) and write (2*1026-1) buffer parts
        lightPosBuf[clipmapLevel][1025u] = -map.height_max;
        lightPosBuf[clipmapLevel][2051u] = -map.height_max;

        // the actual ray position (== workgroup invocation)
        lightPosBuf[clipmapLevel][1u + ray_number] = -map.height_max;
        lightPosBuf[clipmapLevel][1027u + ray_number] = -map.height_max;
    }
}
// ----------------------------------------------------------------------------
// ray direction tracing left <-> right of heightmap
fn calculate_and_update_lightheight_horizontal(
    ray_number: u32, ray_step: u32, lightheight_delta: f32, clipmap_level: u32
) -> LightHeightResult {
    let map_coord = to_map_coord_horizontal(ray_number, ray_step);

    let result =
        calculate_lightheight(clipmap_level, ray_number, ray_step, lightheight_delta, map_coord);

    textureStore(lightmap, map_coord, i32(clipmap_level), vec4<u32>(result.lightmapHeight));

    return result;
}
// ----------------------------------------------------------------------------
// ray direction tracing top <-> bottom of heightmap
fn calculate_and_update_lightheight_vertical(
    ray_number: u32, ray_step: u32, lightheight_delta: f32, clipmap_level: u32
) -> LightHeightResult {
    let map_coord = to_map_coord_vertical(ray_number, ray_step);

    let result =
        calculate_lightheight(clipmap_level, ray_number, ray_step, lightheight_delta, map_coord);

    textureStore(lightmap, map_coord, i32(clipmap_level), vec4<u32>(result.lightmapHeight));

    return result;
}
// ----------------------------------------------------------------------------
fn store_dbg(ray_number: u32, ray_step: u32, clipmap_level: u32, value: u32) {
    let map_coord = to_map_coord_horizontal(ray_number, ray_step);
    textureStore(lightmap, map_coord, i32(clipmap_level), vec4<u32>(value));
}
// ----------------------------------------------------------------------------
// transfers the results of a step between different clipmap layers that were
// involved in this tracing step
fn transfer_step_to_step_info_between_layers(
    step: u32,
    highest_res_clipmap_level: u32,
    lowest_res_clipmap_level: u32,
) {
    // prepare lightpos (down + up clipmap levels)
    for (var clipmap_level = highest_res_clipmap_level; clipmap_level < lowest_res_clipmap_level; clipmap_level = clipmap_level + 1u) {
        // m == higher res level
        let clipmap_level_m = clipmap_level;
        // n == lower res level
        let clipmap_level_n = clipmap_level + 1u;

        let clipmap_step_m = step >> clipmap_level_m;
        let clipmap_step_n = step >> clipmap_level_n;

        let clipmap_stride_m = 1u << clipmap_level_m;
        let clipmap_stride_n = 1u << clipmap_level_n;

        // first ray *in* clipmap
        let first_ray_height = lightPosBuf[clipmap_level_m][writeBuf(clipmap_step_m) + 1u];
        // last ray *in* clipmap
        let last_ray_height = lightPosBuf[clipmap_level_m][writeBuf(clipmap_step_m) + 1024u];

        let first_hidden_ray_n =
            (rayClipmap.layer[clipmap_level_m].ray_1 - rayClipmap.layer[clipmap_level_n].ray_1) >> clipmap_level_n;

        let last_hidden_ray_n =
            (rayClipmap.layer[clipmap_level_m].ray_after - rayClipmap.layer[clipmap_level_n].ray_1) >> clipmap_level_n;

        // always update *both* parts (read + write) of the lightpos buffer to
        // prevent mixups (Note: different layers may use different stepstride!)
        // Note: offset 1 == ray 0
        lightPosBuf[clipmap_level_n][1u    + first_hidden_ray_n] = first_ray_height;
        lightPosBuf[clipmap_level_n][1027u + first_hidden_ray_n] = first_ray_height;

        lightPosBuf[clipmap_level_n][        last_hidden_ray_n] = last_ray_height;
        lightPosBuf[clipmap_level_n][1026u + last_hidden_ray_n] = last_ray_height;

        // move from lower to higher sample
        // last ray before higher res interpolated with first ray hidden by higher res clipmap layer
        // (improves silhouette resolution from very long shadows which originate in lower res layer
        // and are propagate multiple time to higher res layers)
        let last_ray_before_m_height = lightPosBuf[clipmap_level_n][readBuf(clipmap_step_n) + first_hidden_ray_n - 1u];
        let first_ray_in_m_height = lightPosBuf[clipmap_level_n][readBuf(clipmap_step_n) + first_hidden_ray_n];

        // first borderpoint (ray_0 - 1)
        let interpolated = (last_ray_before_m_height + first_ray_in_m_height) * 0.5;
        lightPosBuf[clipmap_level_m][   0u] = interpolated;
        lightPosBuf[clipmap_level_m][1026u] = interpolated;

        // first ray after higher res layer interpolated with last hidden ray by higher res layer
        let last_hidden_ray_m_height = lightPosBuf[clipmap_level_n][readBuf(clipmap_step_n) + last_hidden_ray_n];
        let first_ray_after_m_height = lightPosBuf[clipmap_level_n][readBuf(clipmap_step_n) + last_hidden_ray_n + 1u];

        // last borderpoint (ray_max + 1)
        let interpolated = (last_hidden_ray_m_height + first_ray_after_m_height) * 0.5;
        lightPosBuf[clipmap_level_m][1025u] = interpolated;
        lightPosBuf[clipmap_level_m][2051u] = interpolated;
    }
    // store_dbg(ray_number, ray_step / 16u, clipmap_level_n, 10000u);
}
// ----------------------------------------------------------------------------
// transfers current results down to lower res clipmap border (interpolated) and
// and up to higher res border (interpolated)
// only for last step in a slice!
fn transfer_slice_to_slice_info_between_layers(
    thread_id: u32,
    step: u32,
    highest_res_clipmap_level: u32,
    lowest_res_clipmap_level: u32,
) {
    if (thread_id < 512u) {
        // depending on slice progression next highest res clipmap level will be higher OR lower
        // -> it's either upsampling to higher res level OR downsampling to lower res but never both
        // since the additional info (higher or lower) is not tracked data is transfered in both
        // directions

        // up sampling (n -> m) only if it's not already highest level
        if (highest_res_clipmap_level > 0u) {
            let ray_num = thread_id;

            let clipmap_level_n = highest_res_clipmap_level;
            let clipmap_level_m = highest_res_clipmap_level - 1u;

            let clipmap_step_m = step >> clipmap_level_m;

            let offset_ray_n_m =
                (rayClipmap.layer[clipmap_level_m].ray_1 - rayClipmap.layer[clipmap_level_n].ray_1) >> clipmap_level_n;

            let current_ray_n_offset = writeBuf(clipmap_step_m) + offset_ray_n_m + ray_num;

            let clipmap_n_light_height_prev = lightPosBuf[clipmap_level_n][current_ray_n_offset - 1u];
            let clipmap_n_light_height = lightPosBuf[clipmap_level_n][current_ray_n_offset];
            let clipmap_n_light_height_next = lightPosBuf[clipmap_level_n][current_ray_n_offset + 1u];

            let interpolated_a = (clipmap_n_light_height_prev + clipmap_n_light_height) * 0.5;
            let interpolated_b = (clipmap_n_light_height_next + clipmap_n_light_height) * 0.5;

            // always update *both* parts (read + write) of the lightpos buffer to
            // prevent mixups (Note: different layers may use different stepstride!)
            // Note: offset 1 == ray 0
            let target_ray = ray_num << 1u;
            lightPosBuf[clipmap_level_m][   1u + target_ray] = interpolated_a;
            lightPosBuf[clipmap_level_m][1027u + target_ray] = interpolated_a;
            lightPosBuf[clipmap_level_m][   1u + target_ray + 1u] = interpolated_b;
            lightPosBuf[clipmap_level_m][1027u + target_ray + 1u] = interpolated_b;

            // upper borderpoint in higher res clipmap level
            let clipmap_n_light_height =
                    lightPosBuf[clipmap_level_n][writeBuf(clipmap_step_m) + offset_ray_n_m - 1u];

            // first borderpoint (ray_0 - 1)
            lightPosBuf[clipmap_level_m][   0u] = clipmap_n_light_height;
            lightPosBuf[clipmap_level_m][1026u] = clipmap_n_light_height;

            // lower borderpoint in higher res clipmap level
            let clipmap_n_light_height =
                    lightPosBuf[clipmap_level_n][writeBuf(clipmap_step_m) + offset_ray_n_m + 512u];

            // last borderpoint (ray_max + 1)
            lightPosBuf[clipmap_level_m][1025u] = clipmap_n_light_height;
            lightPosBuf[clipmap_level_m][2051u] = clipmap_n_light_height;
        }
    } else {
        // down sample (m -> n)
        if (highest_res_clipmap_level < lowest_res_clipmap_level) {
            let ray_num = thread_id - 512u;

            let clipmap_level_n = highest_res_clipmap_level + 1u;
            let clipmap_level_m = highest_res_clipmap_level;

            let clipmap_step_m = step >> clipmap_level_m;

            let src_ray_offset = writeBuf(clipmap_step_m) + (ray_num << 1u);
            let lightheight_1 = lightPosBuf[clipmap_level_m][src_ray_offset];
            let lightheight_2 = lightPosBuf[clipmap_level_m][src_ray_offset + 1u];

            let clipmap_m_light_height = (lightheight_1 + lightheight_2) * 0.5;

            let offset_ray_n_m =
                (rayClipmap.layer[clipmap_level_m].ray_1 - rayClipmap.layer[clipmap_level_n].ray_1) >> clipmap_level_n;

            // always update *both* parts (read + write) of the lightpos buffer to
            // prevent mixups (Note: different layers may use different stepstride!)
            // Note: offset 1 == ray 0
            lightPosBuf[clipmap_level_n][   0u + offset_ray_n_m + ray_num] = clipmap_m_light_height;
            lightPosBuf[clipmap_level_n][1026u + offset_ray_n_m + ray_num] = clipmap_m_light_height;
        }
    }
}
// ----------------------------------------------------------------------------
// full size downsampling of clipmap layers
fn downsample_clipmaps(
    thread_id: u32,
    highest_res_clipmap_level: u32,
    lowest_res_clipmap_level: u32,
) {
    // split thread id into two components (32x32)
    let thread_x = thread_id & 31u;
    let thread_y = (thread_id >> 5u) & 31u;

    for (var clipmap_level = highest_res_clipmap_level; clipmap_level < lowest_res_clipmap_level; clipmap_level = clipmap_level + 1u) {
        let clipmap_level_src = clipmap_level;
        let clipmap_level_target = clipmap_level_src + 1u;

        let offset_target = (clipmap.layers[clipmap_level_src].map_offset - clipmap.layers[clipmap_level_target].map_offset);
        let offset_target = vec2<i32>(i32(offset_target.x >> clipmap_level_target), i32(offset_target.y >> clipmap_level_target));

        // downsample 32x32 areas to 16x16
        for (var y = 0u; y < 16u; y = y + 1u) {
            for (var x = 0u; x < 16u; x = x + 1u) {

                let target_coord = vec2<i32>(i32((thread_x << 4u) + x), i32((thread_y << 4u) + y));

                let v1 = textureLoad(lightmap, target_coord * 2, i32(clipmap_level_src));
                let v2 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(0, 1), i32(clipmap_level_src));
                let v3 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 0), i32(clipmap_level_src));
                let v4 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 1), i32(clipmap_level_src));
                let downsampled = (v1.x + v2.x + v3.x + v4.x) >> 2u;

                textureStore(lightmap, offset_target + target_coord, i32(clipmap_level_target), vec4<u32>(downsampled));
            }
        }
        storageBarrier();
    }
}
// ----------------------------------------------------------------------------
// downsamples only a portion of higher res clipmap layer: border strip of high
// res clipmap layer that is overlaying the lower res clipmap level.
// required to smooth out trnasition between clipmap layers if some rounding
// errors occur which may lead to selecting "wrong" layer (== lower res layer
// instead of higher res layer).
fn downsample_clipmaps_strip(
    thread_id: u32,
    highest_res_clipmap_level: u32,
    lowest_res_clipmap_level: u32,
) {
    // downsample only a slim border frame (4x4) to lower res clipmap level in
    // order to cover one-off access to clipmap level
    //
    // every thread covers onyl 4x4 of higher res level
    //      aaaaaa 0..
    //      b....c
    //      b....c
    //      b....c
    //      b....c
    //      dddddd
    //
    // TODO: corner points are processed multiple times -> isolate threads?

    // split thread id into four groups a 256
    var thread_x: u32;
    var thread_y: u32;

    // TODO calculate directly from bits?
    if (thread_id < 512u) {
        if (thread_id < 256u) {
            thread_x = thread_id;
            thread_y = 0u;
        } else {
            thread_x = (thread_id - 256u);
            thread_y = 255u;
        }
    } else {
        if (thread_id < 768u) {
            thread_x = 0u;
            thread_y = (thread_id - 512u);
        } else {
            thread_x = 255u;
            thread_y = (thread_id - 768u);
        }
    }

    for (var clipmap_level = highest_res_clipmap_level; clipmap_level < lowest_res_clipmap_level; clipmap_level = clipmap_level + 1u) {
        let clipmap_level_src = clipmap_level;
        let clipmap_level_target = clipmap_level_src + 1u;

        let offset_target = (clipmap.layers[clipmap_level_src].map_offset - clipmap.layers[clipmap_level_target].map_offset);
        let offset_target = vec2<i32>(i32(offset_target.x >> clipmap_level_target), i32(offset_target.y >> clipmap_level_target));

        let target_coord = vec2<i32>(i32(thread_x << 1u), i32(thread_y << 1u));
        let v1 = textureLoad(lightmap, target_coord * 2, i32(clipmap_level_src));
        let v2 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(0, 1), i32(clipmap_level_src));
        let v3 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 0), i32(clipmap_level_src));
        let v4 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 1), i32(clipmap_level_src));
        textureStore(lightmap, offset_target + target_coord, i32(clipmap_level_target), vec4<u32>((v1.x + v2.x + v3.x + v4.x) >> 2u));

        let target_coord = vec2<i32>(i32((thread_x << 1u)), i32((thread_y << 1u) + 1u));
        let v1 = textureLoad(lightmap, target_coord * 2, i32(clipmap_level_src));
        let v2 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(0, 1), i32(clipmap_level_src));
        let v3 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 0), i32(clipmap_level_src));
        let v4 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 1), i32(clipmap_level_src));
        textureStore(lightmap, offset_target + target_coord, i32(clipmap_level_target), vec4<u32>((v1.x + v2.x + v3.x + v4.x) >> 2u));

        let target_coord = vec2<i32>(i32((thread_x << 1u) + 1u), i32((thread_y << 1u)));
        let v1 = textureLoad(lightmap, target_coord * 2, i32(clipmap_level_src));
        let v2 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(0, 1), i32(clipmap_level_src));
        let v3 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 0), i32(clipmap_level_src));
        let v4 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 1), i32(clipmap_level_src));
        textureStore(lightmap, offset_target + target_coord, i32(clipmap_level_target), vec4<u32>((v1.x + v2.x + v3.x + v4.x) >> 2u));

        let target_coord = vec2<i32>(i32((thread_x << 1u) + 1u), i32((thread_y << 1u) + 1u));
        let v1 = textureLoad(lightmap, target_coord * 2, i32(clipmap_level_src));
        let v2 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(0, 1), i32(clipmap_level_src));
        let v3 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 0), i32(clipmap_level_src));
        let v4 = textureLoad(lightmap, target_coord * 2 + vec2<i32>(1, 1), i32(clipmap_level_src));
        textureStore(lightmap, offset_target + target_coord, i32(clipmap_level_target), vec4<u32>((v1.x + v2.x + v3.x + v4.x) >> 2u));
    }
    storageBarrier();
}
// ----------------------------------------------------------------------------
// specialized entry points depending on ray tracing directions
// Note: a merge beween directions is possible but after some time the code gets
// more complicated to reason about.
// ----------------------------------------------------------------------------
[[stage(compute), workgroup_size(1024, 1, 1)]]
fn main([[builtin(local_invocation_id)]] invocation_id: vec3<u32>) {
    let thread_id = invocation_id.x;
    let lowest_res_clipmap_level = computeSlices.count_and_lowest_level >> 16u;

    // Note: 1024 threads required!
    // since highest layer is always 0 the number of layers is lowest_res_clipmap_level + 1
    init_last_lightpos(thread_id, lowest_res_clipmap_level + 1u);
    storageBarrier();

    // ------------------------------------------------------------------------
    // main slice loop:
    // calculate all steps in a slice until next slice with a *different* number
    // of enclosed clipmap levels is entered
    // ------------------------------------------------------------------------
    // light is assumed as directional: height delta is contant per step
    let ray_height_delta_value = settings.ray_height_delta_per_step;

    let slice_count = computeSlices.count_and_lowest_level & 31u;

    // highest resolution step counter
    var current_ray_step = 0;

    for (var s = 0u; s < slice_count; s = s + 1u) {

        let slice = computeSlices.slice[s];
        let highest_res_clipmap_level = slice.highest_res_clipmap_level;
        let step_after = i32(slice.step_after);

        // defined by highest res clipmap level of current slice
        let slice_step_stride = (1u << highest_res_clipmap_level);

        // number of steps until next slice
        // Note: abs to account for tracing in both directions
        // (current_ray_step may be > step_after if direction is right -> left)
        let slice_steps = u32(abs(step_after - current_ray_step)) / slice_step_stride;

        // --------------------------------------------------------------------
        // schedule for a thread
        // calculate a number of rays for a number of steps (step number is
        // defined by highest res clipmap level in this slice)
        // --------------------------------------------------------------------
        let schedule = threadSchedule.slice[slice.schedule_id][thread_id];
        // Assumption: work of any thread will NOT cross clipmap boundary
        //  -> reason: clipmap pos granularity && assigned (sub)rays per thread
        var thread_start_ray = schedule.start_ray;
        let clipmap_level = schedule.clipmap_level;

        // adjust ray height delta step amount by current level gridstride
        let ray_height_delta = ray_height_delta_value * f32(1u << clipmap_level);

        // transform global step to local step of clipmap level for current thread
        let clipmap_start_step =
            (u32(current_ray_step) - rayClipmap.layer[clipmap_level].step_1) / slice_step_stride;

        for (var step = 0u; step < slice_steps; step = step + 1u) {
            // TODO skip if step % proper clipmap stride != 0

            # ifdef MAIN_DIRECTION
            // trace direction left -> right or top -> bottom
            let thread_step = clipmap_start_step + step >> (clipmap_level - highest_res_clipmap_level);
            # else
            // stride direction right -> left or bottom -> top
            // Note: 1023u is last pos in clipmap (for clipmap size 1024)
            let thread_step = 1023u - (clipmap_start_step + step >> (clipmap_level - highest_res_clipmap_level));
            # endif

            for (var r = 0u; r < schedule.rays; r = r + 1u) {
                // calculate the light height for step + ray + clipmap
                # ifdef HORIZONTAL_RAYS
                calculate_and_update_lightheight_horizontal(
                    (thread_start_ray + r), thread_step, ray_height_delta, clipmap_level);
                # else
                calculate_and_update_lightheight_vertical(
                    (thread_start_ray + r), thread_step, ray_height_delta, clipmap_level);
                # endif
            }
            // sync step results before transfer to other clipmap level
            storageBarrier();

            // prepare lightpos (down + up clipmap levels)
            if (thread_id == 0u) {
                transfer_step_to_step_info_between_layers(step, highest_res_clipmap_level, lowest_res_clipmap_level);
            }
            // sync transfer before progressing with next step
            storageBarrier();
        }
        current_ray_step = step_after;

        // at last step of slice transfer current values down to lower res clipmap border (interpolated)
        // and up to higher res border (interpolated?)
        transfer_slice_to_slice_info_between_layers(thread_id, step, highest_res_clipmap_level, lowest_res_clipmap_level);

        // sync after transfer between layers and before progressing with next slice
        storageBarrier();
    }
    // --------------------------------------------------------------------------------------------
    // final downsampling of highest res -> lowest res clipmap at appropriate positions
    let highest_res_clipmap_level = 0u;

    // downsample_clipmaps(thread_id, highest_res_clipmap_level, lowest_res_clipmap_level);
    downsample_clipmaps_strip(thread_id, highest_res_clipmap_level, lowest_res_clipmap_level);
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
