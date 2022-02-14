// ----------------------------------------------------------------------------
#[cfg(debug_assertions)]
const TERRAIN_TEXTURE_MIP_LEVELS: Option<u8> = None;
#[cfg(not(debug_assertions))]
const TERRAIN_TEXTURE_MIP_LEVELS: Option<u8> = Some(0);
// ----------------------------------------------------------------------------
use bevy::{
    ecs::schedule::StateData, prelude::*, reflect::TypeUuid, render::render_resource::TextureFormat,
};

use crate::texturearray::{TextureArray, TextureArrayBuilder};
use crate::DefaultResources;
// ----------------------------------------------------------------------------
#[derive(Default, Clone, TypeUuid)]
#[uuid = "867a207f-7ada-4fdd-8319-df7d383fa6ff"]
/// handles to all diffuse and normal textures and parameter settings
pub struct TerrainMaterialSet {
    pub diffuse: Handle<TextureArray>,
    pub normal: Handle<TextureArray>,
    pub parameter: [TerrainMaterialParam; 31],
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum TextureType {
    Diffuse,
    Normal,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialSlot(u8);
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct TerrainMaterialParam {
    pub blend_sharpness: f32,
    pub slope_base_dampening: f32,
    pub slope_normal_dampening: f32,
    pub specularity_scale: f32,
    pub specularity: f32,
    pub specularity_base: f32,
    pub _specularity_scale_copy: f32,
    pub falloff: f32,
}
// ----------------------------------------------------------------------------
pub struct MaterialSetPlugin;
// ----------------------------------------------------------------------------
impl MaterialSetPlugin {
    // ------------------------------------------------------------------------
    /// normal active cam operation
    pub fn setup_default_materialset<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(setup_default_materialset)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for MaterialSetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMaterialSet>();
    }
}
// ----------------------------------------------------------------------------
fn setup_default_materialset(
    placeholder: Res<DefaultResources>,
    textures: Res<Assets<Image>>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    mut materialset: ResMut<TerrainMaterialSet>,
) {
    use TextureFormat::Rgba8UnormSrgb;
    debug!("generating default material pallete.start");

    let logo = textures.get(&placeholder.logo).expect("loaded logo");

    // create placeholder texture arrays
    let dim = 1024;

    // let default_color = [128u8, 128u8, 128u8, 128u8];
    let default_color = [255u8, 255u8, 255u8, 255u8];
    let default_normal = [0u8, 0u8, 255u8, 0u8];

    let mut placeholder_diffuse = default_color.repeat(dim * dim);
    let placeholder_normal = default_normal.repeat(dim * dim);
    // add logo

    let logo_width_bytes = (logo.texture_descriptor.size.width * 4) as usize;
    for y in 0..logo.texture_descriptor.size.height as usize {
        let logo_offset = y * logo_width_bytes;
        let logo_slice = &logo.data[logo_offset..logo_offset + logo_width_bytes];

        let offset = 4 * (y * dim + 200);
        placeholder_diffuse[offset..offset + logo_width_bytes].copy_from_slice(logo_slice);
    }

    let dim = dim as u32;
    let placeholder_diffuse = image::DynamicImage::ImageRgba8(
        image::ImageBuffer::from_raw(dim, dim, placeholder_diffuse).unwrap(),
    );
    let placeholder_normal = image::DynamicImage::ImageRgba8(
        image::ImageBuffer::from_raw(dim, dim, placeholder_normal).unwrap(),
    );

    let mut diffuse_array =
        TextureArrayBuilder::new(dim, 31, Rgba8UnormSrgb, TERRAIN_TEXTURE_MIP_LEVELS);
    let mip_sizes = diffuse_array.mip_sizes();
    let placeholder_diffuse = TextureArray::generate_mips(placeholder_diffuse, &mip_sizes);
    let placeholder_normal = TextureArray::generate_mips(placeholder_normal, &mip_sizes);

    for _ in 1..=31 {
        diffuse_array.add_texture_with_mips(placeholder_diffuse.clone());
        // diffuse_array.add_texture(placeholder_diffuse.clone());
    }
    materialset.diffuse = texture_arrays.add(diffuse_array.build());

    let mut normal_array =
        TextureArrayBuilder::new(dim as u32, 31, Rgba8UnormSrgb, TERRAIN_TEXTURE_MIP_LEVELS);
    for _ in 1..=31 {
        normal_array.add_texture_with_mips(placeholder_normal.clone());
        // normal_array.add_texture(placeholder_normal.clone());
    }
    materialset.normal = texture_arrays.add(normal_array.build());

    debug!("generating default material pallete.end");
}
// ----------------------------------------------------------------------------
// material params
// ----------------------------------------------------------------------------
impl Default for TerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // TODO check defaults
        Self {
            blend_sharpness: 0.0,
            slope_base_dampening: 0.0,
            slope_normal_dampening: 0.5,
            specularity_scale: 0.0,
            specularity: 0.0,
            specularity_base: 0.0,
            _specularity_scale_copy: 0.0,
            falloff: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<u8> for MaterialSlot {
    fn from(v: u8) -> Self {
        MaterialSlot(v)
    }
}
// ----------------------------------------------------------------------------
impl std::ops::Deref for MaterialSlot {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
use std::ops::{Index, IndexMut};

impl Index<MaterialSlot> for [TerrainMaterialParam; 31] {
    type Output = TerrainMaterialParam;

    fn index(&self, index: MaterialSlot) -> &Self::Output {
        &self[*index as usize]
    }
}
// ----------------------------------------------------------------------------
impl IndexMut<MaterialSlot> for [TerrainMaterialParam; 31] {
    fn index_mut(&mut self, index: MaterialSlot) -> &mut Self::Output {
        &mut self[*index as usize]
    }
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// fmt
// ----------------------------------------------------------------------------
use std::fmt;

impl fmt::Display for TextureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureType::Diffuse => write!(f, "diffuse"),
            TextureType::Normal => write!(f, "normal"),
        }
    }
}
// ----------------------------------------------------------------------------
impl fmt::Display for MaterialSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
// ----------------------------------------------------------------------------
