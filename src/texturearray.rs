// ----------------------------------------------------------------------------
// based on bevy_render/src/texture/image.rs
// ----------------------------------------------------------------------------
pub struct TextureArrayPlugin;
// ----------------------------------------------------------------------------
#[derive(Debug, TypeUuid)]
#[uuid = "b8783fe3-4169-41ea-8d4d-db23c40e4ee9"]
pub struct TextureArray {
    // TODO: this nesting makes accessing Image metadata verbose. Either flatten out descriptor or add accessors
    pub texture_descriptor: TextureDescriptor<'static>,
    pub sampler_descriptor: SamplerDescriptor<'static>,

    mip_level: Vec<TextureArrayMipLevel>,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct TextureMipLevel {
    size: u32,
    data: Vec<u8>,
}
// ----------------------------------------------------------------------------
pub struct TextureArrayBuilder {
    added_layers: usize,
    array: TextureArray,
}
// ----------------------------------------------------------------------------
/// Lanczos3 is slightrly slower but mips are visibily less blurred than Triangle
const MIP_FILTER: FilterType = FilterType::Lanczos3;
// const MIP_FILTER: FilterType = FilterType::Nearest;
// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::Size,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
        render_resource::{
            AddressMode, Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout, Origin3d,
            SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{GpuImage, TextureFormatPixelInfo},
    },
};
use image::{imageops::FilterType, DynamicImage};
// ----------------------------------------------------------------------------
impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenderAssetPlugin::<TextureArray>::default())
            .add_asset::<TextureArray>();
    }
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
struct TextureArrayMipLevel {
    size: u32,
    textures: Vec<Vec<u8>>,
}
// ----------------------------------------------------------------------------
impl TextureArrayMipLevel {
    // ------------------------------------------------------------------------
    fn new(size: u32, texture_count: u32) -> Self {
        Self {
            size,
            textures: vec![Vec::default(); texture_count as usize],
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TextureArray {
    // ------------------------------------------------------------------------
    /// highest_mip_level defines the lowest res mip level that will be auto
    /// generated. providing 0 as highest_mip_level will auto generates all
    /// levels up to one pixel. set to None if only mip 0 should be used.
    fn new(size: Extent3d, format: TextureFormat, highest_mip_level: Option<u8>) -> Self {
        #[rustfmt::skip]
        assert!(size.width == size.height, "arbitrary sized rectangles not supported");
        // #[rustfmt::skip]
        // assert!(size.width.is_power_of_two(), "only power of two for size supported");

        // important: mip0 must alsways exist!
        let mip_level = Self::calculate_mip_sizes(size.width, highest_mip_level)
            .iter()
            .map(|mip_size| TextureArrayMipLevel::new(*mip_size, size.depth_or_array_layers))
            .collect::<Vec<_>>();

        Self {
            texture_descriptor: TextureDescriptor {
                size,
                format,
                dimension: TextureDimension::D2,
                label: None,
                // mip0 also counts
                mip_level_count: mip_level.len() as u32,

                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            },
            sampler_descriptor: SamplerDescriptor {
                mipmap_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                ..Default::default()
            },
            mip_level,
        }
    }
    // ------------------------------------------------------------------------
    pub fn generate_mips(data: DynamicImage, mip_sizes: &[u32]) -> Vec<TextureMipLevel> {
        let mut result = Vec::with_capacity(mip_sizes.len());
        let mut source = data;

        // Note: always reducing from max res pic does not have any visible
        // (subjective) quality advantage
        let max_mip = mip_sizes.len() - 1;
        for (i, mip_size) in mip_sizes.iter().copied().enumerate() {
            if i < max_mip {
                let next_size = mip_size >> 1;
                let next_mip = source.resize_exact(next_size, next_size, MIP_FILTER);

                result.push(TextureMipLevel {
                    size: mip_size,
                    data: source.into_bytes(),
                });
                source = next_mip;
            } else {
                // last level
                result.push(TextureMipLevel {
                    size: mip_size,
                    data: source.into_bytes(),
                });
                break;
            }
        }
        result
    }
    // ------------------------------------------------------------------------
    pub fn update_slot_with_mips(&mut self, slot: u8, mut data: Vec<TextureMipLevel>) {
        assert!(self.mip_level.len() == data.len());

        for (i, (destination, source)) in self.mip_level.iter_mut().zip(data.drain(..)).enumerate()
        {
            assert!(
                destination.size == source.size,
                "mip {} destination.size != source.size: {} != {}",
                i,
                destination.size,
                source.size
            );
            destination.textures[slot as usize] = source.data;
        }
    }
    // ------------------------------------------------------------------------
    /// highest_mip_level defines the lowest res mip level that will be auto
    /// generated. providing 0 as highest_mip_level will auto generates all
    /// levels up to one pixel. set to None if only mip 0 should be used.
    pub fn calculate_mip_sizes(size: u32, highest_mip_level: Option<u8>) -> Vec<u32> {
        let max_mip_level = if let Some(mip) = highest_mip_level {
            // f32 workaround until feature 'int_log' is stable
            let max_miplevel = (size as f32).log2() as u32;
            if mip == 0 {
                max_miplevel
            } else {
                (mip as u32).min(max_miplevel)
            }
        } else {
            0
        };
        (0..=max_mip_level).map(|l| size >> l).collect::<Vec<_>>()
    }
    // ------------------------------------------------------------------------
    pub fn mip_sizes(&self) -> Vec<u32> {
        self.mip_level.iter().map(|mip| mip.size).collect()
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn update_slot(&mut self, slot: u8, data: DynamicImage) {
        use image::{ColorType, GenericImageView};

        let d = &self.texture_descriptor;

        assert!(d.size.depth_or_array_layers > slot as u32);
        assert!(data.dimensions() == (d.size.width, d.size.height));

        match self.texture_descriptor.format {
            TextureFormat::R16Uint => assert!(data.color() == ColorType::L16),
            TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Sint
            | TextureFormat::Rgba8Uint => assert!(data.color() == ColorType::Rgba8),
            _ => todo!("unsupported texture array format"),
        };

        let mip_sizes = self.mip_sizes();
        self.update_slot_with_mips(slot, Self::generate_mips(data, &mip_sizes));
    }
    // ------------------------------------------------------------------------
    pub fn texture_count(&self) -> u32 {
        self.texture_descriptor.size.depth_or_array_layers
    }
    // ------------------------------------------------------------------------
    pub fn imagedata(&self, slot: u8, request_size: u32) -> (TextureFormat, u32, &[u8]) {
        assert!(self.texture_descriptor.size.depth_or_array_layers > slot as u32);

        let data = self
            .mip_level
            .iter()
            .reduce(|accum, current| {
                if current.size >= request_size {
                    current
                } else {
                    accum
                }
            })
            .map(|mip| {
                // every mip level has all textures, slot was already asserted
                (
                    self.texture_descriptor.format,
                    mip.size,
                    mip.textures[slot as usize].as_slice(),
                )
            })
            .unwrap();
        data
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TextureArrayBuilder {
    // ------------------------------------------------------------------------
    /// highest_mip_level defines the lowest res mip level that will be auto
    /// generated. providing 0 as highest_mip_level will auto generates all
    /// levels up to one pixel. set to None if only mip 0 should be used.
    pub fn new(
        size: u32,
        layers: u32,
        format: TextureFormat,
        highest_mip_level: Option<u8>,
    ) -> Self {
        assert!(layers > 1, "texture arrays require at least 2 layers");

        Self {
            added_layers: 0,
            array: TextureArray::new(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: layers,
                },
                format,
                highest_mip_level,
            ),
        }
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn mip_sizes(&self) -> Vec<u32> {
        self.array.mip_sizes()
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn add_texture(&mut self, data: DynamicImage) {
        let slot = self.added_layers as u8;
        self.array.update_slot(slot, data);
        self.added_layers += 1;
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn add_texture_with_mips(&mut self, data: Vec<TextureMipLevel>) {
        let slot = self.added_layers as u8;
        self.array.update_slot_with_mips(slot, data);
        self.added_layers += 1;
    }
    // ------------------------------------------------------------------------
    pub fn add_usage(mut self, usages: TextureUsages) -> Self {
        self.array.texture_descriptor.usage |= usages;
        self
    }
    // ------------------------------------------------------------------------
    pub fn build(self) -> TextureArray {
        #[rustfmt::skip]
        assert!(self.added_layers as u32 == self.array.texture_descriptor.size.depth_or_array_layers);

        self.array
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// render world preparation
// ----------------------------------------------------------------------------
pub struct ExtractedTextureArray {
    mips: Vec<Vec<u8>>,
    texture_descriptor: TextureDescriptor<'static>,
    sampler_descriptor: SamplerDescriptor<'static>,
}
// ----------------------------------------------------------------------------
impl RenderAsset for TextureArray {
    type ExtractedAsset = ExtractedTextureArray;
    type PreparedAsset = GpuImage;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);
    // ------------------------------------------------------------------------
    fn extract_asset(&self) -> Self::ExtractedAsset {
        let mip0_size = &self.texture_descriptor.size;

        let mut mips = Vec::new();

        let pixel_size = self.texture_descriptor.format.pixel_size();
        // combine mips into one array
        for (level, mip) in self.mip_level.iter().enumerate() {
            let size = if level > 0 {
                let size = mip0_size.mip_level_size(level as u32, false);
                (size.width * size.height * size.depth_or_array_layers) as usize * pixel_size
            } else {
                (mip0_size.width * mip0_size.height * mip0_size.depth_or_array_layers) as usize
                    * pixel_size
            };

            let mut combined_mip_level = Vec::with_capacity(size);

            for texture in &mip.textures {
                combined_mip_level.extend_from_slice(texture);
            }

            mips.push(combined_mip_level);
        }
        ExtractedTextureArray {
            mips,
            texture_descriptor: self.texture_descriptor.clone(),
            sampler_descriptor: self.sampler_descriptor.clone(),
        }
    }
    // ------------------------------------------------------------------------
    fn prepare_asset(
        texture_array: Self::ExtractedAsset,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        use std::num::NonZeroU32;

        let texture = render_device.create_texture(&texture_array.texture_descriptor);
        let sampler = render_device.create_sampler(&texture_array.sampler_descriptor);

        let format_size = texture_array.texture_descriptor.format.pixel_size();

        for (level, mipdata) in texture_array.mips.iter().enumerate() {
            let size = if level > 0 {
                texture_array
                    .texture_descriptor
                    .size
                    .mip_level_size(level as u32, false)
            } else {
                texture_array.texture_descriptor.size
            };

            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                mipdata,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(NonZeroU32::new(size.width * format_size as u32).unwrap()),
                    rows_per_image: if size.depth_or_array_layers > 1 {
                        NonZeroU32::new(size.height)
                    } else {
                        None
                    },
                },
                size,
            );
        }

        let texture_view = texture.create_view(&TextureViewDescriptor {
            mip_level_count: NonZeroU32::new(texture_array.mips.len() as u32),
            ..Default::default()
        });
        let size = Size::new(
            texture_array.texture_descriptor.size.width as f32,
            texture_array.texture_descriptor.size.height as f32,
        );
        Ok(GpuImage {
            texture,
            texture_view,
            sampler,
            size,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
