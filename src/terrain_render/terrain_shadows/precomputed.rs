// ----------------------------------------------------------------------------
use crate::config::CLIPMAP_SIZE;

use super::{
    ClipmapLayerInfo, ComputeSliceInfo, ComputeThreadJob, DirectionalClipmapLayerInfo,
    LightrayDirection, TerrainShadowsComputeInput, TerrainShadowsComputeTrigger,
    TerrainShadowsLightrayInfo,
};
// ----------------------------------------------------------------------------
impl TerrainShadowsComputeTrigger {
    // ------------------------------------------------------------------------
    pub(super) fn inactive() -> Self {
        Self {
            recompute: false,
            trace_direction: LightrayDirection::LeftRight,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for TerrainShadowsLightrayInfo {
    fn default() -> Self {
        Self {
            lightpos_offset: 0,
            interpolation_weight: 0.0,
            ray_height_delta: 0.0,
            direction: LightrayDirection::LeftRight,
        }
    }
}
// ----------------------------------------------------------------------------
impl ComputeThreadJob {
    // ------------------------------------------------------------------------
    fn new(ray_count: u8, start_ray: u16, clipmap_level: u8) -> Self {
        Self {
            rays: ray_count as u32,
            start_ray: start_ray as u32,
            clipmap_level: clipmap_level as u32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ComputeSliceInfo {
    // ------------------------------------------------------------------------
    fn new(highest_clipmap_level: u8, step_after_slice: u16, schedule_id: usize) -> Self {
        Self {
            highest_res_clipmap_level: highest_clipmap_level as u32,
            step_after: step_after_slice as u32,
            schedule_id: schedule_id as u32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl DirectionalClipmapLayerInfo {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn contains_ray(&self, ray: u16) -> bool {
        ray >= self.ray_1 && ray < self.ray_after
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainShadowsComputeInput {
    // ------------------------------------------------------------------------
    pub(super) fn recalculate_schedule(&mut self, lightray_info: &TerrainShadowsLightrayInfo) {
        //
        // all threads have to be executed in the same workgroup to share data
        // and be synchronize step after step. it is assumed 1024 threads are
        // available for workgroup.x
        //
        // TODO maybe make the thread count somewhat dynamic
        // https://vulkan.gpuinfo.org/displaydevicelimit.php?platform=windows&name=maxComputeWorkGroupSize[0]
        //
        // assert!(CLIPMAP_SIZE < 1024);

        let map_size = self.map_size;
        let layerinfo = self
            .clipmap_info
            .info
            .iter()
            .map(|l| DirectionalClipmapLayerInfo::from((map_size, l, lightray_info.direction)))
            .collect::<Vec<_>>();

        // simple version:
        // same number of rays is assigned to every thread (until all rays are
        // assigend). to ensure that rays assigned to a thread *always* reside
        // in the same clipmap level the ray count has to be a power of two.
        // this is also means that depending on layer count some threads will
        // have *NO* work at all.
        // for a clipmap size of 1024 and a position granularity of 256 and
        // subsequent clipmap layer convering double the size, e.g.
        //  layer 0: rectangle of 1024x1024 -> 1024x1024 (full res)
        //  layer 1: rectangle of 2048x2048 -> 1024x1024 (half res)
        //  layer 2: rectangle of 4096x4096 -> 1024x1024 (quarter res)
        //  ...
        // assuming 3 level clipmap every slice will cover either 1, 2 or 3
        // clipmap level:
        //          --------------------
        //          | a   b  c  d   e  |        a -> clipmap level 2
        //          |    ----------    |        b -> clipmap level 2, 1
        //          |    |        |    |        c -> clipmap level 2, 1, 0
        //          |    |  ----  |    |        d -> clipmap level 2, 1
        //          |    |  |  |  |    |        e -> clipmap level 2
        //          |    |  ----  |    |
        //          |    |        |    |
        //          |    ----------    |
        //          |                  |
        //          --------------------
        //
        // since for every level 1024 (== clipmap size) rays need to be traced
        // slices have different sum of rays to be traced:
        //  slice a -> 1024     // 1024 level 2
        //  slice b -> 1536     // 512 (level 2) + 1024 (level 1)
        //  slice c -> 2048     // 512 (level 2) + 512 (level 1) + 1024 (level 0)
        //  slice d -> 1536     // 512 (level 2) + 1024 (level 1)
        //  slice e -> 1024     // 1024 level 2
        //
        // to divide work more or less equally a valid raycount for slices would
        // be:
        //  slice a -> 1 ray per thread     // 1024 threads have a 1-ray-job
        //  slice b -> 2 ray per thread     //  768 threads have a 2-rays-job
        //  slice c -> 2 ray per thread     // 1024 threads have a 2-rays-job
        //  slice d -> 2 ray per thread     //  768 threads have a 2-rays-job
        //  slice e -> 1 ray per thread     // 1024 threads have a 1-rays-job
        //
        // TODO it may be faster to use different number of rays per thread to
        //  use all available threads, though more complicated to calculate

        // TODO these can be cached in a hashmap
        self.thread_jobs = match layerinfo.len() {
            1 => vec![generate_jobs_for_1_level(0, &layerinfo)],
            2 => vec![
                generate_jobs_for_1_level(1, &layerinfo),
                generate_jobs_for_2_level(0, &layerinfo),
            ],
            3 => vec![
                generate_jobs_for_1_level(2, &layerinfo),
                generate_jobs_for_2_level(1, &layerinfo),
                generate_jobs_for_3_level(0, &layerinfo),
            ],
            4 => vec![
                generate_jobs_for_1_level(3, &layerinfo),
                generate_jobs_for_2_level(2, &layerinfo),
                generate_jobs_for_3_level(1, &layerinfo),
                generate_jobs_for_4_level(0, &layerinfo),
            ],
            5 => vec![
                generate_jobs_for_1_level(4, &layerinfo),
                generate_jobs_for_2_level(3, &layerinfo),
                generate_jobs_for_3_level(2, &layerinfo),
                generate_jobs_for_4_level(1, &layerinfo),
                generate_jobs_for_5_level(0, &layerinfo),
            ],
            _ => unreachable!(),
        };

        self.compute_slices = match layerinfo.len() {
            1 => vec![ComputeSliceInfo::new(0, layerinfo[0].step_after, 0)],
            2 => vec![
                ComputeSliceInfo::new(1, layerinfo[0].step_1, 0),
                ComputeSliceInfo::new(0, layerinfo[0].step_after, 1),
                ComputeSliceInfo::new(1, layerinfo[1].step_after, 0),
            ],
            3 => vec![
                ComputeSliceInfo::new(2, layerinfo[1].step_1, 0),
                ComputeSliceInfo::new(1, layerinfo[0].step_1, 1),
                ComputeSliceInfo::new(0, layerinfo[0].step_after, 2),
                ComputeSliceInfo::new(1, layerinfo[1].step_after, 1),
                ComputeSliceInfo::new(2, layerinfo[2].step_after, 0),
            ],
            4 => vec![
                ComputeSliceInfo::new(3, layerinfo[2].step_1, 0),
                ComputeSliceInfo::new(2, layerinfo[1].step_1, 1),
                ComputeSliceInfo::new(1, layerinfo[0].step_1, 2),
                ComputeSliceInfo::new(0, layerinfo[0].step_after, 3),
                ComputeSliceInfo::new(1, layerinfo[1].step_after, 2),
                ComputeSliceInfo::new(2, layerinfo[2].step_after, 1),
                ComputeSliceInfo::new(3, layerinfo[3].step_after, 0),
            ],
            5 => vec![
                ComputeSliceInfo::new(4, layerinfo[3].step_1, 0),
                ComputeSliceInfo::new(3, layerinfo[2].step_1, 1),
                ComputeSliceInfo::new(2, layerinfo[1].step_1, 2),
                ComputeSliceInfo::new(1, layerinfo[0].step_1, 3),
                ComputeSliceInfo::new(0, layerinfo[0].step_after, 4),
                ComputeSliceInfo::new(1, layerinfo[1].step_after, 3),
                ComputeSliceInfo::new(2, layerinfo[2].step_after, 2),
                ComputeSliceInfo::new(3, layerinfo[3].step_after, 1),
                ComputeSliceInfo::new(4, layerinfo[4].step_after, 0),
            ],
            _ => unreachable!(),
        };

        self.directional_clipmap_layer = layerinfo;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn generate_jobs_for_5_level(
    start_layer: usize,
    layers: &[DirectionalClipmapLayerInfo],
) -> Vec<ComputeThreadJob> {
    let mut jobs = Vec::with_capacity(CLIPMAP_SIZE as usize);

    let ray_count = 4;
    let max_rays = (CLIPMAP_SIZE * (1 << (start_layer + 4) as u32)) as u16;

    let mut ray_number = 0;
    for _ in 0..CLIPMAP_SIZE {
        if ray_number >= max_rays {
            jobs.push(ComputeThreadJob::new(0, 0, 0));
        } else {
            let clipmap = if layers[start_layer].contains_ray(ray_number) {
                start_layer
            } else if layers[start_layer + 1].contains_ray(ray_number) {
                start_layer + 1
            } else if layers[start_layer + 2].contains_ray(ray_number) {
                start_layer + 2
            } else if layers[start_layer + 3].contains_ray(ray_number) {
                start_layer + 3
            } else {
                start_layer + 4
            };

            let stride = 1 << clipmap;
            jobs.push(ComputeThreadJob::new(
                ray_count as u8,
                (ray_number - layers[clipmap as usize].ray_1) / stride,
                clipmap as u8,
            ));
            ray_number += ray_count * stride;
        }
    }
    jobs
}
// ----------------------------------------------------------------------------
fn generate_jobs_for_4_level(
    start_layer: usize,
    layers: &[DirectionalClipmapLayerInfo],
) -> Vec<ComputeThreadJob> {
    let mut jobs = Vec::with_capacity(CLIPMAP_SIZE as usize);

    let ray_count = 4;
    let max_rays = (CLIPMAP_SIZE * (1 << (start_layer + 3) as u32)) as u16;

    let mut ray_number = 0;
    for _ in 0..CLIPMAP_SIZE {
        if ray_number >= max_rays {
            jobs.push(ComputeThreadJob::new(0, 0, 0));
        } else {
            let clipmap = if layers[start_layer].contains_ray(ray_number) {
                start_layer
            } else if layers[start_layer + 1].contains_ray(ray_number) {
                start_layer + 1
            } else if layers[start_layer + 2].contains_ray(ray_number) {
                start_layer + 2
            } else {
                start_layer + 3
            };

            let stride = 1 << clipmap;
            jobs.push(ComputeThreadJob::new(
                ray_count as u8,
                (ray_number - layers[clipmap as usize].ray_1) / stride,
                clipmap as u8,
            ));
            ray_number += ray_count * stride;
        }
    }
    jobs
}
// ----------------------------------------------------------------------------
fn generate_jobs_for_3_level(
    start_layer: usize,
    layers: &[DirectionalClipmapLayerInfo],
) -> Vec<ComputeThreadJob> {
    let mut jobs = Vec::with_capacity(CLIPMAP_SIZE as usize);

    let ray_count = 2;
    let max_rays = (CLIPMAP_SIZE * (1 << (start_layer + 2) as u32)) as u16;

    let mut ray_number = 0;
    for _ in 0..CLIPMAP_SIZE {
        if ray_number >= max_rays {
            jobs.push(ComputeThreadJob::new(0, 0, 0));
        } else {
            let clipmap = if layers[start_layer].contains_ray(ray_number) {
                start_layer
            } else if layers[start_layer + 1].contains_ray(ray_number) {
                start_layer + 1
            } else {
                start_layer + 2
            };

            let stride = 1 << clipmap;
            jobs.push(ComputeThreadJob::new(
                ray_count as u8,
                (ray_number - layers[clipmap as usize].ray_1) / stride,
                clipmap as u8,
            ));
            ray_number += ray_count * stride;
        }
    }
    jobs
}
// ----------------------------------------------------------------------------
fn generate_jobs_for_2_level(
    start_layer: usize,
    layers: &[DirectionalClipmapLayerInfo],
) -> Vec<ComputeThreadJob> {
    let mut jobs = Vec::with_capacity(CLIPMAP_SIZE as usize);

    let ray_count = 2;
    let max_rays = (CLIPMAP_SIZE * (1 << (start_layer + 1) as u32)) as u16;

    let mut ray_number = 0;
    for _ in 0..CLIPMAP_SIZE {
        if ray_number >= max_rays {
            jobs.push(ComputeThreadJob::new(0, 0, 0));
        } else {
            let clipmap = if layers[start_layer].contains_ray(ray_number) {
                start_layer
            } else {
                start_layer + 1
            };

            let stride = 1 << clipmap;
            jobs.push(ComputeThreadJob::new(
                ray_count as u8,
                (ray_number - layers[clipmap as usize].ray_1) / stride,
                clipmap as u8,
            ));
            ray_number += ray_count * stride;
        }
    }
    jobs
}
// ----------------------------------------------------------------------------
fn generate_jobs_for_1_level(
    start_layer: usize,
    layers: &[DirectionalClipmapLayerInfo],
) -> Vec<ComputeThreadJob> {
    let mut jobs = Vec::with_capacity(CLIPMAP_SIZE as usize);

    let ray_count = 1;
    let max_rays = (CLIPMAP_SIZE * (1 << start_layer as u32)) as u16;

    let mut ray_number = 0;
    for _ in 0..CLIPMAP_SIZE {
        if ray_number >= max_rays {
            jobs.push(ComputeThreadJob::new(0, 0, 0));
        } else {
            let clipmap = start_layer;

            let stride = 1 << clipmap;
            jobs.push(ComputeThreadJob::new(
                ray_count as u8,
                (ray_number - layers[clipmap as usize].ray_1) / stride,
                clipmap as u8,
            ));
            ray_number += ray_count * stride;
        }
    }
    jobs
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl<'a> From<(u32, &'a ClipmapLayerInfo, LightrayDirection)> for DirectionalClipmapLayerInfo {
    // ------------------------------------------------------------------------
    fn from((map_size, info, direction): (u32, &'a ClipmapLayerInfo, LightrayDirection)) -> Self {
        use LightrayDirection::*;

        let info_size = info.size as u32;

        match direction {
            LeftRight => Self {
                step_1: info.map_offset.x as u16,
                step_after: (info.map_offset.x + info_size) as u16,
                ray_1: info.map_offset.y as u16,
                ray_after: (info.map_offset.y + info_size) as u16,
            },
            RightLeft => Self {
                // steps are now from right to left !
                step_1: (map_size - info.map_offset.x - info_size) as u16,
                step_after: (map_size - info.map_offset.x) as u16,
                ray_1: info.map_offset.y as u16,
                ray_after: (info.map_offset.y + info_size) as u16,
            },
            TopBottom => Self {
                step_1: info.map_offset.y as u16,
                step_after: (info.map_offset.y + info_size) as u16,
                ray_1: info.map_offset.x as u16,
                ray_after: (info.map_offset.x + info_size) as u16,
            },
            BottomTop => Self {
                // steps are now from bottom to top !
                step_1: (map_size - info.map_offset.y - info_size) as u16,
                step_after: (map_size - info.map_offset.y) as u16,
                ray_1: info.map_offset.x as u16,
                ray_after: (info.map_offset.x + info_size) as u16,
            },
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
