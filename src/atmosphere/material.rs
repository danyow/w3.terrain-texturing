// ----------------------------------------------------------------------------
// ported from bevy_atmosphere:
//  https://github.com/JonahPlusPlus/bevy_atmosphere
//  by Jonah Henriksson
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
/// Controls the appearance of the sky
///
/// Due to constraints on the shader, namely the number of uniforms in a set being capped off at 8,
/// some fields were combined, therefore, functions are provided to set individual fields
#[derive(Debug, TypeUuid, Clone, AsStd140)]
#[uuid = "9c6670f3-931e-4b51-85c2-17dcc5c6a5ec"]
pub struct AtmosphereMat {
    /// Default: (0.0, 6372e3, 0.0)
    ray_origin: Vec3,
    /// Default: (0.0, 1.0, 1.0)
    sun_position: Vec3,
    /// Default: 22.0
    sun_intensity: f32,
    /// Represents Planet radius (Default: 6371e3) and Atmosphere radius (Default: 6471e3)
    radius: Vec2,
    /// Represents Rayleigh coefficient (Default: (5.5e-6, 13.0e-6, 22.4e-6)) and scale height
    /// (Default: 8e3)
    rayleigh: Vec4,
    /// Represents Mie coefficient (Default: 21e-6), scale height (Default: 1.2e3) and preferred
    /// scattering direction (Default: 0.758)
    mie: Vec3,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct GpuAtmosphereMat {
    _buffer: Buffer,
    bind_group: BindGroup,
}
// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::{Vec2, Vec3, Vec4},
    pbr::{MaterialPipeline, SpecializedMaterial},
    prelude::{AssetServer, Handle, Shader},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            RenderPipelineDescriptor, ShaderStages, SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
    },
};
// ----------------------------------------------------------------------------
#[allow(dead_code)]
impl AtmosphereMat {
    // ------------------------------------------------------------------------
    pub fn ray_origin(&self) -> Vec3 {
        self.ray_origin
    }
    // ------------------------------------------------------------------------
    pub fn sub_position(&self) -> Vec3 {
        self.sun_position
    }
    // ------------------------------------------------------------------------
    pub fn sun_intensity(&self) -> f32 {
        self.sun_intensity
    }
    // ------------------------------------------------------------------------
    pub fn planet_radius(&self) -> f32 {
        self.radius.x
    }
    // ------------------------------------------------------------------------
    pub fn atmosphere_radius(&self) -> f32 {
        self.radius.y
    }
    // ------------------------------------------------------------------------
    pub fn rayleigh_scattering_coefficient(&self) -> Vec3 {
        Vec3::new(self.rayleigh.x, self.rayleigh.y, self.rayleigh.z)
    }
    // ------------------------------------------------------------------------
    pub fn rayleigh_scale_height(&self) -> f32 {
        self.rayleigh.w
    }
    // ------------------------------------------------------------------------
    pub fn mie_scattering_coefficient(&self) -> f32 {
        self.mie.x
    }
    // ------------------------------------------------------------------------
    pub fn mie_scale_height(&self) -> f32 {
        self.mie.y
    }
    // ------------------------------------------------------------------------
    pub fn mie_scattering_direction(&self) -> f32 {
        self.mie.z
    }
    // ------------------------------------------------------------------------
    /// Sets the ray origin
    pub fn set_ray_origin(&mut self, ray_origin: Vec3) {
        self.ray_origin = ray_origin;
    }
    // ------------------------------------------------------------------------
    /// Sets the sun's position
    pub fn set_sun_position(&mut self, sun_position: Vec3) {
        self.sun_position = sun_position;
    }
    // ------------------------------------------------------------------------
    /// Sets the sun's intensity (brightness)
    pub fn set_sun_intensity(&mut self, sun_intensity: f32) {
        self.sun_intensity = sun_intensity;
    }
    // ------------------------------------------------------------------------
    /// Sets the planet's radius (in meters)
    pub fn set_planet_radius(&mut self, planet_radius: f32) {
        self.radius.x = planet_radius;
    }
    // ------------------------------------------------------------------------
    /// Sets the atmosphere's radius (in meters)
    pub fn set_atmosphere_radius(&mut self, atmosphere_radius: f32) {
        self.radius.y = atmosphere_radius;
    }
    // ------------------------------------------------------------------------
    /// Sets the Rayleigh scattering coefficient
    pub fn set_rayleigh_scattering_coefficient(&mut self, coefficient: Vec3) {
        self.rayleigh.x = coefficient.x;
        self.rayleigh.y = coefficient.y;
        self.rayleigh.z = coefficient.z;
    }
    // ------------------------------------------------------------------------
    /// Sets the scale height (in meters) for Rayleigh scattering
    pub fn set_rayleigh_scale_height(&mut self, scale: f32) {
        self.rayleigh.w = scale;
    }
    // ------------------------------------------------------------------------
    /// Sets the Mie scattering coefficient
    pub fn set_mie_scattering_coefficient(&mut self, coefficient: f32) {
        self.mie.x = coefficient;
    }
    // ------------------------------------------------------------------------
    /// Sets the scale height (in meters) for Mie scattering
    pub fn set_mie_scale_height(&mut self, scale: f32) {
        self.mie.y = scale;
    }
    // ------------------------------------------------------------------------
    /// Sets the preferred direction for Mie scattering
    pub fn set_mie_scattering_direction(&mut self, direction: f32) {
        self.mie.z = direction;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for AtmosphereMat {
    fn default() -> Self {
        Self {
            ray_origin: Vec3::new(0.0, 6372e3, 0.0),
            sun_position: Vec3::new(0.0, 1.0, 1.0),
            sun_intensity: 22.0,
            radius: Vec2::new(6371e3, 6471e3),
            rayleigh: Vec4::new(5.5e-6, 13.0e-6, 22.4e-6, 8e3),
            mie: Vec3::new(21e-6, 1.2e3, 0.758),
        }
    }
}
// ----------------------------------------------------------------------------
impl SpecializedMaterial for AtmosphereMat {
    type Key = ();
    // ------------------------------------------------------------------------
    fn key(_: &<AtmosphereMat as RenderAsset>::PreparedAsset) -> Self::Key {}
    // ------------------------------------------------------------------------
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        _layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.entry_point = "main".into();
        descriptor.fragment.as_mut().unwrap().entry_point = "main".into();

        //FIXME doesn't seem to be necessary?
        // if let Some(depth_stencil_state) = descriptor.depth_stencil.as_mut() {
        //     depth_stencil_state.depth_compare = CompareFunction::LessEqual;
        //     depth_stencil_state.depth_write_enabled = false;
        // }
        Ok(())
    }
    // ------------------------------------------------------------------------
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/atmosphere/sky.vert"))
    }
    // ------------------------------------------------------------------------
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/atmosphere/sky.frag"))
    }
    // ------------------------------------------------------------------------
    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }
    // ------------------------------------------------------------------------
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(AtmosphereMat::std140_size_static() as u64),
                },
                count: None,
            }],
            label: None,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl RenderAsset for AtmosphereMat {
    // ------------------------------------------------------------------------
    type ExtractedAsset = AtmosphereMat;
    type PreparedAsset = GpuAtmosphereMat;
    type Param = (SRes<RenderDevice>, SRes<MaterialPipeline<Self>>);
    // ------------------------------------------------------------------------
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: extracted_asset.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuAtmosphereMat {
            _buffer: buffer,
            bind_group,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
