// ----------------------------------------------------------------------------
use bevy::render::render_resource::TextureFormat;

use crate::clipmap::ClipmapData;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TextureControl {
    size: u32,
    data: Vec<u16>,
}
// ----------------------------------------------------------------------------
impl TextureControl {
    // ------------------------------------------------------------------------
    pub fn new(size: u32, data: Vec<u16>) -> Self {
        assert!((size * size) as usize == data.len());
        Self { size, data }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ClipmapData for TextureControl {
    // ------------------------------------------------------------------------
    type DataType = u16;
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn datapoint_size(&self) -> u32 {
        // 16bit
        1
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn texture_format(&self) -> TextureFormat {
        TextureFormat::R16Uint
    }
    // ------------------------------------------------------------------------
    fn wrap_as_image(&self, size: u32, data: Vec<Self::DataType>) -> image::DynamicImage {
        use image::{DynamicImage::ImageLuma16, ImageBuffer};

        ImageLuma16(ImageBuffer::from_raw(size, size, data).unwrap())
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn size(&self) -> u32 {
        self.size
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn as_slice(&self) -> &[Self::DataType] {
        &self.data
    }
    // ------------------------------------------------------------------------
    fn downscale(
        &self,
        src: &[Self::DataType],
        src_size: usize,
        src_x: usize,
        src_y: usize,
        src_roi_size: usize,
        target_size: usize,
    ) -> Vec<Self::DataType> {
        assert!(src_size * src_size == src.len());
        assert!(src_size - src_x >= target_size);
        assert!(src_size - src_y >= target_size);

        // since texture control must not change pixel values only no filtering
        // is allowed to apply -> calculate stride

        let mut result = Vec::with_capacity(target_size * target_size);

        let start_offset = src_y * src_size + src_x;
        let stride = src_roi_size / target_size;

        let mut offset = start_offset;
        for sy in 0..target_size {
            for _sx in 0..target_size {
                result.push(src[offset]);
                offset += stride;
            }
            offset = start_offset + sy * src_size * stride;
        }

        result
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
