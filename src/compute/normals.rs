// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{internal::bytemuck, ComputePipeline},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferAddress,
            BufferAsyncError, BufferBindingType, BufferDescriptor, BufferInitDescriptor,
            BufferSize, BufferSlice, BufferUsages, CommandEncoder, ComputePassDescriptor, MapMode,
            ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp,
    },
};
use futures_lite::Future;

use super::{
    computetask::{ComputeTask, ComputeTaskPlugin},
    CachedShaderId, ComputePipelineCache, ComputePipelineDescriptor, GpuComputeTask,
    SpecializedComputePipeline, SpecializedComputePipelines,
};
// ----------------------------------------------------------------------------
const WORKGROUP_SIZE_X: u32 = 8;
const WORKGROUP_SIZE_Y: u32 = 8;
// ----------------------------------------------------------------------------
pub struct ComputeNormalsPlugin;
// ----------------------------------------------------------------------------
pub struct ComputeNormalsResult {
    pub offset: usize,
    pub normals: Vec<[f32; 3]>,
}
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct AppComputeNormalsTask {
    /// resolution of heightmap: pixel per meter
    pub map_resolution: f32,
    /// u16 heightmap values will be scaled by this factor so result is in meters
    pub map_height_scaling: f32,
    /// heightmap values per row
    pub data_width: u32,
    /// rows to calculate (Note: the input data has 2 rows more!)
    pub data_rows: u32,
    /// helper info for reintegrating compute result into normals data buffer
    /// (only relevant if data_width > data_rows)
    pub data_offset: usize,
    /// asumption: additional two rows of data: first and last row to be used as
    /// previous and next rows for interpolation
    pub data: Option<Vec<u16>>,
}
// ----------------------------------------------------------------------------
impl Plugin for ComputeNormalsPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        // receives the finished data for a task
        app.add_plugin(ComputeTaskPlugin::<AppComputeNormalsTask>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ComputeNormalsPipeline>()
            .init_resource::<SpecializedComputePipelines<ComputeNormalsPipeline>>();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ComputeNormalsPipelineKey;
// ----------------------------------------------------------------------------
pub struct ComputeNormalsPipeline {
    data_layout: BindGroupLayout,

    test_shader: CachedShaderId,
}
// ----------------------------------------------------------------------------
impl FromWorld for ComputeNormalsPipeline {
    fn from_world(world: &mut World) -> Self {
        // hack to add shader
        let mut pipeline_cache = world.get_resource_mut::<ComputePipelineCache>().unwrap();

        let shaderid = pipeline_cache.set_shader(
            Some("compute_normals_shader"),
            include_str!("../../assets/shaders/compute/normals.wgsl"),
        );

        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let data_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute_normals_task_layout"),
            entries: &[
                // params
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            GpuNormalTaskUniform::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                // heightmap
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None, //BufferSize::new(? as u64),
                    },
                    count: None,
                },
                // normals
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None, //BufferSize::new(? as u64),
                    },
                    count: None,
                },
            ],
        });

        Self {
            data_layout,
            test_shader: shaderid,
        }
    }
}
// ----------------------------------------------------------------------------
impl SpecializedComputePipeline for ComputeNormalsPipeline {
    // ------------------------------------------------------------------------
    type Key = ComputeNormalsPipelineKey;
    // ------------------------------------------------------------------------
    fn specialize(&self, _key: Self::Key) -> ComputePipelineDescriptor {
        // no specialization
        ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![self.data_layout.clone()]),
            shader: self.test_shader,
            entry_point: "main".into(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct ExtractedComputeNormalsTask {
    /// resolution of heightmap: pixel per meter
    map_resolution: f32,
    /// u16 heightmap values will be scaled by this factor so result is in meters
    map_height_scaling: f32,
    /// heightmap values per row
    data_width: u32,
    /// rows to calculate (Note: the input data has 2 rows more!)
    data_rows: u32,
    /// helper info for reintegrating compute result into normals data buffer
    /// (only relevant if data_width > data_rows)
    data_offset: usize,
    /// asumption: additional two rows of data: first and last row to be used as
    /// previous and next rows for interpolation
    data: Vec<u16>,
}
// ----------------------------------------------------------------------------
impl ComputeTask for AppComputeNormalsTask {
    type ExtractedComputeTask = ExtractedComputeNormalsTask;
    type ComputePipeline = ComputeNormalsPipeline;
    // ------------------------------------------------------------------------
    fn specialization_key(
        _extracted_task: &Self::ExtractedComputeTask,
    ) -> <Self::ComputePipeline as SpecializedComputePipeline>::Key {
        ComputeNormalsPipelineKey
    }
    // ------------------------------------------------------------------------
    fn extract_task(&mut self) -> Self::ExtractedComputeTask {
        ExtractedComputeNormalsTask {
            map_resolution: self.map_resolution,
            map_height_scaling: self.map_height_scaling,
            data_width: self.data_width,
            data_rows: self.data_rows,
            data_offset: self.data_offset,
            data: self.data.take().unwrap(),
        }
    }
    // ------------------------------------------------------------------------
    fn prepare_task(
        extracted_task: Self::ExtractedComputeTask,
        render_device: &RenderDevice,
        pipeline: &ComputeNormalsPipeline,
        _render_queue: &RenderQueue,
    ) -> GpuComputeTask {
        GpuComputeTask::ComputeNormals(GpuComputeNormals::new(
            GpuNormalTaskUniform {
                map_resolution: extracted_task.map_resolution,
                map_height_scaling: extracted_task.map_height_scaling,
                data_width: extracted_task.data_width,
            },
            extracted_task.data_rows,
            extracted_task.data_offset,
            extracted_task.data,
            render_device,
            pipeline,
        ))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuNormalTaskUniform {
    map_resolution: f32,
    map_height_scaling: f32,
    data_width: u32,
}
// ----------------------------------------------------------------------------
pub struct GpuComputeNormals {
    bind_group: BindGroup,

    _heightmap_buffer: Buffer,
    calculated_normals_buffer: Buffer,
    staging_buffer: Buffer,
    staging_buffer_size: BufferAddress,

    workgroups: (u32, u32),
    offset: usize,
}
// ----------------------------------------------------------------------------
impl GpuComputeNormals {
    // ------------------------------------------------------------------------
    fn new(
        settings: GpuNormalTaskUniform,
        data_rows: u32,
        offset: usize,
        heightmap: Vec<u16>,
        render_device: &RenderDevice,
        pipeline: &ComputeNormalsPipeline,
    ) -> Self {
        // calculate the size of the result buffer
        let result_buf_size =
            (settings.data_width * data_rows) as usize * std::mem::size_of::<f32>() * 3;

        // much faster with additional staging buffer in comparison to map::read for calc_normals_buf
        let result_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: result_buf_size as BufferAddress,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let info_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("normals_info_buffer"),
            usage: BufferUsages::UNIFORM, // | BufferUsages::COPY_DST,
            contents: settings.as_std140().as_bytes(),
        });

        let heightmap_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("heightmap_input_storage_buffer"),
            contents: bytemuck::cast_slice(&heightmap),
            usage: BufferUsages::STORAGE, // | BufferUsages::COPY_SRC,
        });

        let calculated_normals_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("normals_output_storage_buffer"),
            size: result_buf_size as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.data_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: info_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: heightmap_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: calculated_normals_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            bind_group,

            _heightmap_buffer: heightmap_buffer,
            calculated_normals_buffer,
            staging_buffer: result_buffer,
            staging_buffer_size: result_buf_size as BufferAddress,

            workgroups: (
                settings.data_width / WORKGROUP_SIZE_X,
                data_rows / WORKGROUP_SIZE_Y,
            ),
            offset,
        }
    }
    // ------------------------------------------------------------------------
    pub fn record_commands(&self, pipeline: &ComputePipeline, cmd_encoder: &mut CommandEncoder) {
        {
            let mut compute_pass = cmd_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute_pass"),
            });
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            // Number of cells to run, the (x,y,z) size of item being processed
            compute_pass.dispatch(self.workgroups.0, self.workgroups.1, 1);
        }

        cmd_encoder.copy_buffer_to_buffer(
            &self.calculated_normals_buffer,
            0,
            &self.staging_buffer,
            0,
            self.staging_buffer_size,
        );
    }
    // ------------------------------------------------------------------------
    pub fn wait_for_result(
        &self,
    ) -> impl Future<Output = Result<BufferSlice, BufferAsyncError>> + Send {
        let buf_slice = self.staging_buffer.slice(..);
        let buf_future = buf_slice.map_async(MapMode::Read);
        async move {
            buf_future.await?;
            Ok(buf_slice)
        }
    }
    // ------------------------------------------------------------------------
    pub fn get_result<'a>(
        &'a self,
        wait_future: impl Future<Output = Result<BufferSlice<'a>, BufferAsyncError>> + Send,
    ) -> Result<ComputeNormalsResult, BufferAsyncError> {
        use futures_lite::future;

        future::block_on(wait_future).map(|slice| {
            let data = slice.get_mapped_range();
            // contents are got in bytes, this converts these bytes back
            let result: &[f32] = bytemuck::cast_slice(&data);
            let result: Vec<[f32; 3]> =
                result.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();

            drop(data);
            self.staging_buffer.unmap();
            ComputeNormalsResult {
                offset: self.offset,
                normals: result,
            }
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
