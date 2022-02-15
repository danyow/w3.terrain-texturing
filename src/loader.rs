// ----------------------------------------------------------------------------
use std::fs::File;

use futures_lite::Future;

use png::{BitDepth, ColorType};
// ----------------------------------------------------------------------------
pub struct LoaderPlugin;
// ----------------------------------------------------------------------------
impl LoaderPlugin {
    // ------------------------------------------------------------------------
    pub(crate) fn load_terrain_texture(
        filepath: String,
        size: u32,
    ) -> impl Future<Output = Result<image::RgbaImage, String>> {
        use png::{BitDepth::Eight, ColorType::Rgba};

        async move {
            let data = Self::load_png_data(Rgba, Eight, size, &filepath)?;
            Ok(image::RgbaImage::from_raw(size, size, data).unwrap())
        }
    }
    // ------------------------------------------------------------------------
    pub fn load_png_data(
        colortype: ColorType,
        bitdepth: BitDepth,
        resolution: u32,
        filepath: &str,
    ) -> Result<Vec<u8>, String> {
        use png::{Decoder, Transformations};

        let file =
            File::open(filepath).map_err(|e| format!("failed to open file {}: {}", filepath, e))?;

        let mut decoder = Decoder::new(file);
        decoder.set_transformations(Transformations::IDENTITY);

        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("failed to decode png file {}: {}", filepath, e))?;

        let mut img_data = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut img_data)
            .map_err(|e| format!("failed to read image format info for: {}: {}", filepath, e))?;

        if info.color_type != colortype || info.bit_depth != bitdepth {
            return Err(format!(
                "file {}: format must be {:?}-Bit {:?}. found {:?}-Bit {:?}",
                filepath, bitdepth, colortype, info.bit_depth, info.color_type
            ));
        }
        if info.width != resolution || info.height != resolution {
            return Err(format!(
                "file {}: expected width x height to be {} x {}. found: {} x {}",
                filepath, resolution, resolution, info.width, info.height
            ));
        }

        Ok(img_data)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
