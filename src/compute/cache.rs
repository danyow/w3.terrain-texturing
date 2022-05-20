// ----------------------------------------------------------------------------
// pretty much cloned (and "simplified") from RenderPipelineCache because it
// doesn't support ComputePipelineDescriptor
// ----------------------------------------------------------------------------
pub struct SpecializedComputePipelines<S: SpecializedComputePipeline> {
    cache: HashMap<S::Key, CachedPipelineId>,
}
// ----------------------------------------------------------------------------
impl<S: SpecializedComputePipeline> Default for SpecializedComputePipelines<S> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}
// ----------------------------------------------------------------------------
impl<S: SpecializedComputePipeline> SpecializedComputePipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut ComputePipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue(descriptor)
        })
    }
}
// ----------------------------------------------------------------------------
pub trait SpecializedComputePipeline {
    type Key: Clone + Hash + PartialEq + Eq + Send + Sync;
    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor;
}
// ----------------------------------------------------------------------------
use std::{borrow::Cow, hash::Hash, ops::Deref, sync::Arc};

use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutId, ComputePipeline, PipelineLayout,
            PipelineLayoutDescriptor, RawComputePipelineDescriptor, ShaderModule,
            ShaderModuleDescriptor, ShaderSource,
        },
        renderer::RenderDevice,
    },
    utils::{HashMap, HashSet},
};
// ----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedPipelineId(usize);

impl CachedPipelineId {
    #[allow(unused)]
    pub const INVALID: Self = CachedPipelineId(usize::MAX);
}
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct ComputePipelineDescriptor {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Option<Vec<BindGroupLayout>>,
    /// The compiled shader module for this stage.
    // pub shader: Handle<Shader>,
    pub shader: CachedShaderId,
    /// The name of the entry point in the compiled shader. There must be a function that returns
    /// void with this name in the shader.
    pub entry_point: Cow<'static, str>,
}
// ----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedShaderId(usize);
// ----------------------------------------------------------------------------
#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<Vec<BindGroupLayoutId>, PipelineLayout>,
}
// ----------------------------------------------------------------------------
impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
    ) -> &PipelineLayout {
        let key = bind_group_layouts.iter().map(|l| l.id()).collect();
        self.layouts.entry(key).or_insert_with(|| {
            let bind_group_layouts = bind_group_layouts
                .iter()
                .map(|l| l.value())
                .collect::<Vec<_>>();
            render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &bind_group_layouts,
                ..Default::default()
            })
        })
        //  render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
        //     label: Some("compute"),
        //     bind_group_layouts: &[&data_layout],
        //     push_constant_ranges: &[],
        // });
    }
}
// ----------------------------------------------------------------------------
// slightly simplified version of RenderPipelineCache
pub struct ComputePipelineCache {
    layout_cache: LayoutCache,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: HashSet<CachedPipelineId>,
    // FIXME using raw schader txt as string key to skip whole shadercache impl copy...
    shaders_ids: HashMap<String, CachedShaderId>,
    shaders: Vec<Arc<ShaderModule>>,
}
// ----------------------------------------------------------------------------
struct CachedPipeline {
    descriptor: ComputePipelineDescriptor,
    state: CachedPipelineState,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum CachedPipelineState {
    Queued,
    Ok(ComputePipeline),
    // Err(RenderPipelineError),
}
// ----------------------------------------------------------------------------
// impl CachedPipelineState {
//     pub fn unwrap(&self) -> &ComputePipeline {
//         match self {
//             CachedPipelineState::Ok(pipeline) => pipeline,
//             CachedPipelineState::Queued => {
//                 panic!("Pipeline has not been compiled yet. It is still in the 'Queued' state.")
//             } // CachedPipelineState::Err(err) => panic!("{}", err),
//         }
//     }
// }
// ----------------------------------------------------------------------------
impl ComputePipelineCache {
    // ------------------------------------------------------------------------
    pub fn new(device: RenderDevice) -> Self {
        Self {
            device,
            layout_cache: Default::default(),
            pipelines: Default::default(),
            waiting_pipelines: Default::default(),

            shaders: Default::default(),
            shaders_ids: Default::default(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn set_shader(
        &mut self,
        label: Option<&str>,
        compute_shader_source_wgsl: &str,
    ) -> CachedShaderId {
        if let Some(id) = self.shaders_ids.get(compute_shader_source_wgsl) {
            *id
        } else {
            let id = CachedShaderId(self.shaders.len());
            let compute_shader = self.device.create_shader_module(&ShaderModuleDescriptor {
                label,
                source: ShaderSource::Wgsl(compute_shader_source_wgsl.into()),
            });
            self.shaders_ids
                .insert(compute_shader_source_wgsl.into(), id);
            self.shaders.push(Arc::new(compute_shader));
            id
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn get(&self, id: CachedPipelineId) -> Option<&ComputePipeline> {
        if let CachedPipelineState::Ok(pipeline) = &self.pipelines[id.0].state {
            Some(pipeline)
        } else {
            None
        }
    }
    // ------------------------------------------------------------------------
    pub fn queue(&mut self, descriptor: ComputePipelineDescriptor) -> CachedPipelineId {
        let id = CachedPipelineId(self.pipelines.len());
        self.pipelines.push(CachedPipeline {
            descriptor,
            state: CachedPipelineState::Queued,
        });
        self.waiting_pipelines.insert(id);
        id
    }
    // ------------------------------------------------------------------------
    pub fn process_queue(&mut self) {
        let pipelines = std::mem::take(&mut self.waiting_pipelines);
        for id in pipelines {
            let state = &mut self.pipelines[id.0];
            match &state.state {
                CachedPipelineState::Ok(_) => continue,
                CachedPipelineState::Queued => {}
                // CachedPipelineState::Err(err) => {
                //     match err {
                //         RenderPipelineError::ShaderNotLoaded(_)
                //         | RenderPipelineError::ShaderImportNotYetAvailable => { /* retry */ }
                //         // shader could not be processed ... retrying won't help
                //         RenderPipelineError::ProcessShaderError(err) => {
                //             error!("failed to process shader: {}", err);
                //             continue;
                //         }
                //         RenderPipelineError::AsModuleDescriptorError(err, source) => {
                //             log_shader_error(source, err);
                //             continue;
                //         }
                //     }
                // }
            }

            let descriptor = &state.descriptor;

            let compute_module = &self.shaders[descriptor.shader.0];

            let layout = if let Some(layout) = &descriptor.layout {
                Some(self.layout_cache.get(&self.device, layout))
            } else {
                None
            };

            let descriptor = RawComputePipelineDescriptor {
                label: descriptor.label.as_deref(),
                layout,
                module: compute_module,
                entry_point: descriptor.entry_point.deref(),
            };

            let pipeline = self.device.create_compute_pipeline(&descriptor);
            state.state = CachedPipelineState::Ok(pipeline);
        }
    }
    // ------------------------------------------------------------------------
    pub fn process_pipeline_queue_system(mut cache: ResMut<Self>) {
        cache.process_queue();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
