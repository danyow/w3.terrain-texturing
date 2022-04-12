// ----------------------------------------------------------------------------
use bevy::render::render_resource::TextureFormat;

use crate::clipmap::ClipmapData;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TintMap {
    size: u32,
    data: Vec<u8>,
}
// ----------------------------------------------------------------------------
impl TintMap {
    // ------------------------------------------------------------------------
    pub fn new(size: u32, data: Vec<u8>) -> Self {
        assert!((size * size * 4) as usize == data.len());
        Self { size, data }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<TintMap> for Vec<u8> {
    // ------------------------------------------------------------------------
    fn from(t: TintMap) -> Self {
        t.data
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ClipmapData for TintMap {
    type DataType = u8;
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn datapoint_size(&self) -> u32 {
        // 8bit RGBA
        4
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn texture_format(&self) -> TextureFormat {
        TextureFormat::Rgba8Unorm
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
    #[inline(always)]
    fn as_slice_mut(&mut self) -> &mut [Self::DataType] {
        &mut self.data
    }
    // ------------------------------------------------------------------------
    fn wrap_as_image(&self, size: u32, data: Vec<Self::DataType>) -> image::DynamicImage {
        use image::{DynamicImage::ImageRgba8, ImageBuffer};

        ImageRgba8(ImageBuffer::from_raw(size, size, data).unwrap())
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
        let px_size = self.datapoint_size() as usize;

        assert!(src_size * src_size * px_size == src.len());
        assert!(src_size - src_x >= target_size);
        assert!(src_size - src_y >= target_size);

        // since texture control must not change pixel values only no filtering
        // is allowed to apply -> calculate stride
        let mut result = Vec::with_capacity(target_size * target_size * px_size);

        let start_offset = px_size * (src_y * src_size + src_x);
        let stride = px_size * (src_roi_size / target_size);

        let mut offset = start_offset;
        for sy in 0..target_size {
            for _sx in 0..target_size {
                result.extend_from_slice(&src[offset..offset + 4]);
                offset += stride;
            }
            offset = start_offset + sy * src_size * stride;
        }

        result
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
