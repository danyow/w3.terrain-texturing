// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            internal::bytemuck,
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferAddress,
            BufferAsyncError, BufferDescriptor, BufferInitDescriptor, BufferUsages,
            CachedRenderPipelineId, Maintain, MapMode, PipelineCache, SpecializedRenderPipelines,
        },
        renderer::RenderDevice,
        view::ExtractedView,
        RenderWorld,
    },
};

use super::{
    BrushPointer, BrushPointerEventData, BrushPointerPipelineKey, BrushPointerRenderPipeline,
    BrushPointerResultDispatcher,
};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct BrushPointerPipelineId(Option<CachedRenderPipelineId>);
// ----------------------------------------------------------------------------
// gpu representation of pointer
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
pub(super) struct GpuBrushPointerInfo {
    cam_pos: Vec3,
    radius: f32,
    pos: Vec2,
    ring_width: f32,
    max_visibility: f32,
    color: Vec4,
    button: u32,
}
// ----------------------------------------------------------------------------
pub(super) struct GpuBrushPointer {
    _buffer: Buffer,
    pub bind_group: BindGroup,
    pub request_result: bool,
}
// ----------------------------------------------------------------------------
// systems (extract, prepare, queue)
// ----------------------------------------------------------------------------
pub(super) fn extract_brush_pointer_info(
    brush_pointer: Res<BrushPointer>,
    mut render_world: ResMut<RenderWorld>,
) {
    if brush_pointer.is_changed() {
        render_world.insert_resource(brush_pointer.clone());
    }
}
// ----------------------------------------------------------------------------
pub(super) fn prepare_brush_pointer_info(
    render_device: Res<RenderDevice>,
    render_pipeline: Res<BrushPointerRenderPipeline>,
    brush_pointer: Res<BrushPointer>,
    mut gpu_brush_pointer: ResMut<Option<GpuBrushPointer>>,
    view: Query<&ExtractedView>,
) {
    if brush_pointer.active {
        let view = view.get_single().unwrap();

        let button = match (brush_pointer.click_primary, brush_pointer.click_secondary) {
            (true, false) => 1,
            (false, true) => 2,
            _ => 0,
        };

        let info_buffer = GpuBrushPointerInfo {
            cam_pos: view.transform.translation,
            pos: brush_pointer.pos,
            radius: brush_pointer.radius,
            ring_width: brush_pointer.ring_width,
            color: Vec4::from(brush_pointer.color.as_rgba_f32()),
            max_visibility: brush_pointer.max_visibility,
            button,
        };

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("brushpointer_info_buffer"),
            contents: info_buffer.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("brushpointer_info_bind_group"),
            layout: &render_pipeline.info_layout,
        });

        *gpu_brush_pointer = Some(GpuBrushPointer {
            bind_group,
            request_result: brush_pointer.click_primary || brush_pointer.click_secondary,
            _buffer: buffer,
        });
    } else {
        *gpu_brush_pointer = None;
    }
}
// ----------------------------------------------------------------------------
pub(super) fn queue_brush_pointer_info(
    brush_pointer_pipeline: Res<BrushPointerRenderPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BrushPointerRenderPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    gpu_brush: Res<Option<GpuBrushPointer>>,
    mut pipeline_id: ResMut<BrushPointerPipelineId>,
) {
    if let Some(gpu_brush) = (*gpu_brush).as_ref() {
        let key = BrushPointerPipelineKey::from_brush(gpu_brush);
        pipeline_id.0 =
            Some(pipelines.specialize(&mut pipeline_cache, &brush_pointer_pipeline, key));
    }
}
// ----------------------------------------------------------------------------
// gpu representation of pointer interacton result
// ----------------------------------------------------------------------------
pub(super) struct GpuBrushPointerResult {
    pub bind_group: BindGroup,

    pub result_buffer: Buffer,
    pub staging_buffer: Buffer,
    pub staging_buffer_size: BufferAddress,
}
// ----------------------------------------------------------------------------
// systems for pointer interaction result
// ----------------------------------------------------------------------------
pub(super) fn prepare_brush_pointer_result(
    render_device: Res<RenderDevice>,
    render_pipeline: Res<BrushPointerRenderPipeline>,
    mut gpu_brush_pointer_result: ResMut<Option<GpuBrushPointerResult>>,
) {
    if gpu_brush_pointer_result.is_none() {
        // calculate the size of the result buffer
        //TODO for now only a Vec4
        let result_buf_size = Vec4::std140_size_static();

        let staging_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: result_buf_size as BufferAddress,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let result_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("brushpointer_result_storage_buffer"),
            size: result_buf_size as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: result_buffer.as_entire_binding(),
            }],
            label: Some("brushpointer_result_bind_group"),
            layout: &render_pipeline.result_layout,
        });

        *gpu_brush_pointer_result = Some(GpuBrushPointerResult {
            bind_group,
            result_buffer,
            staging_buffer,
            staging_buffer_size: result_buf_size as BufferAddress,
        });
    }
}
// ----------------------------------------------------------------------------
pub(super) fn check_brush_pointer_result(
    render_device: Res<RenderDevice>,
    gpu_brush_pointer_info: Res<Option<GpuBrushPointer>>,
    gpu_brush_pointer_result: Res<Option<GpuBrushPointerResult>>,
    dispatcher: Res<BrushPointerResultDispatcher>,
) {
    if (*gpu_brush_pointer_info)
        .as_ref()
        .map(|b| b.request_result)
        .unwrap_or_default()
    {
        if let Some(pointer_result) = gpu_brush_pointer_result.as_ref() {
            let buf_slice = pointer_result.staging_buffer.slice(..);
            let buf_future = buf_slice.map_async(MapMode::Read);
            let x = async move {
                buf_future.await?;
                Ok(buf_slice)
            };

            // Note: device.poll has to be called *after* wait_for_result (slice.map_async)
            // and before get_result!
            render_device.poll(Maintain::Wait);

            use futures_lite::future;
            let result: Result<BrushPointerEventData, BufferAsyncError> =
                future::block_on(x).map(|slice| {
                    let data = slice.get_mapped_range();
                    // contents are got in bytes, this converts these bytes back
                    let result: &[f32] = bytemuck::cast_slice(&data);
                    let result = BrushPointerEventData::Centered(
                        // button
                        to_mouse_button(result[3] as u8),
                        // position
                        Vec2::from((result[0], result[1])),
                        // radius
                        result[2],
                    );

                    drop(data);
                    pointer_result.staging_buffer.unmap();
                    result
                });

            match result {
                Ok(result) => {
                    dispatcher
                        .0
                        .try_send(result)
                        .map_err(|e| error!("failed to send brush pointer operation result: {}", e))
                        .ok();
                }
                Err(e) => error!("failed to extract brush pointer result: {}", e),
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn to_mouse_button(button: u8) -> MouseButton {
    // since result is only requested if left or right button was pressed any
    // other values *should* not be passed
    if button == 1 {
        MouseButton::Left
    } else {
        MouseButton::Right
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for BrushPointerPipelineId {
    type Target = Option<CachedRenderPipelineId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
// default
// ----------------------------------------------------------------------------
impl Default for BrushPointer {
    fn default() -> Self {
        Self {
            active: false,
            pos: Vec2::ZERO,
            radius: 1.0,
            ring_width: 0.5,
            max_visibility: 4000.0,
            color: Color::YELLOW,
            click_primary: false,
            click_secondary: false,
        }
    }
}
// ----------------------------------------------------------------------------
