// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, ComputePipelineDescriptor,
            SpecializedComputePipeline,
        },
        renderer::RenderDevice,
    },
};

use super::LightrayDirection;
use super::{
    compute_input_bind_group_layout, lightheightmap_bind_group_layout,
    lightray_settings_bind_group_layout,
};
// ----------------------------------------------------------------------------
pub struct ComputeShadowsPipeline {
    shader: Handle<Shader>,
    pub lightheightmap_layout: BindGroupLayout,
    pub input_layout: BindGroupLayout,
    pub lightray_layout: BindGroupLayout,
}
// ----------------------------------------------------------------------------
impl FromWorld for ComputeShadowsPipeline {
    // ------------------------------------------------------------------------
    fn from_world(world: &mut World) -> Self {
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/compute/terrain_shadows.wgsl");

        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let lightheightmap_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("compute_terrain_lightheight_clipmap_layout"),
                entries: &lightheightmap_bind_group_layout(),
            });

        let input_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute_terrain_lightheight_input_layout"),
            entries: &compute_input_bind_group_layout(),
        });

        let lightray_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute_terrain_lightheight_rayinfo_layout"),
            entries: &lightray_settings_bind_group_layout(),
        });

        Self {
            shader,
            lightheightmap_layout,
            input_layout,
            lightray_layout,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ComputeShadowsPipelineKey: u32 {
        const DEFAULT         = 0b0000;
        const MAIN_DIRECTION  = 0b0001;
        const HORIZONTAL_RAYS = 0b0010;
    }
}
// ----------------------------------------------------------------------------
impl ComputeShadowsPipelineKey {
    // ------------------------------------------------------------------------
    pub(super) fn from(trace_direction: LightrayDirection) -> Self {
        match trace_direction {
            LightrayDirection::LeftRight => Self::HORIZONTAL_RAYS | Self::MAIN_DIRECTION,
            LightrayDirection::RightLeft => Self::HORIZONTAL_RAYS,
            LightrayDirection::TopBottom => Self::MAIN_DIRECTION,
            LightrayDirection::BottomTop => Self::DEFAULT,
        }
    }
    // ------------------------------------------------------------------------
    fn shader_defs(&self) -> Vec<String> {
        if self.contains(Self::MAIN_DIRECTION) {
            if self.contains(Self::HORIZONTAL_RAYS) {
                vec!["MAIN_DIRECTION".to_string(), "HORIZONTAL_RAYS".to_string()]
            } else {
                vec!["MAIN_DIRECTION".to_string()]
            }
        } else if self.contains(Self::HORIZONTAL_RAYS) {
            vec!["HORIZONTAL_RAYS".to_string()]
        } else {
            vec![]
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SpecializedComputePipeline for ComputeShadowsPipeline {
    type Key = ComputeShadowsPipelineKey;
    // ------------------------------------------------------------------------
    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        // specialize based on trace direction
        ComputePipelineDescriptor {
            label: Some("compute_terrain_shadow_pipeline".into()),
            layout: Some(vec![
                self.lightheightmap_layout.clone(),
                self.input_layout.clone(),
                self.lightray_layout.clone(),
            ]),
            shader: self.shader.clone(),
            entry_point: "main".into(),
            shader_defs: key.shader_defs(),
        }
    }
}
// ----------------------------------------------------------------------------
