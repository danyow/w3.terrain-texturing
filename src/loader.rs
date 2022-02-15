// ----------------------------------------------------------------------------
use std::fs::File;

use bevy::prelude::*;

use futures_lite::Future;

use png::{BitDepth, ColorType};

use crate::config;
use crate::heightmap::TerrainHeightMap;
use crate::TaskResultData;
// ----------------------------------------------------------------------------
pub struct LoaderPlugin;
// ----------------------------------------------------------------------------
impl LoaderPlugin {
    // ------------------------------------------------------------------------
    pub(crate) fn load_heightmap(
        config: &config::TerrainConfig,
    ) -> impl Future<Output = Result<TaskResultData, String>> {
        use byteorder::{BigEndian, ReadBytesExt};
        use png::{BitDepth::Sixteen, ColorType::Grayscale};
        use std::io::Cursor;

        let (filepath, size, height_scaling) = (
            config.heightmap().to_string(),
            config.map_size(),
            config.height_scaling(),
        );
        async move {
            let data = if filepath.is_empty() {
                debug!("generating heightmap...");
                // generate some terrain as placeholder
                generate_placeholder_heightmap(size)
            } else {
                debug!("loading heightmap...");
                let img_data = Self::load_png_data(Grayscale, Sixteen, size, &filepath)?;

                // transform buffer into 16 bits
                let mut buffer_u16 = vec![0; (size * size) as usize];
                let mut buffer_cursor = Cursor::new(img_data);
                buffer_cursor
                    .read_u16_into::<BigEndian>(&mut buffer_u16)
                    .map_err(|e| format!("failed to convert buffer into u16 values: {}", e))?;

                buffer_u16
            };

            let heightmap = TerrainHeightMap::new(size, height_scaling, data);
            Ok(TaskResultData::HeightmapData(heightmap))
        }
    }
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
#[allow(dead_code)]
fn generate_placeholder_heightmap(gen_size: u32) -> Vec<u16> {
    let mut generated_heightmap = Vec::with_capacity((gen_size * gen_size) as usize);
    for y in 0..gen_size {
        for x in 0..gen_size {
            let scale = 7.0 / gen_size as f32 * (gen_size as f32 / 256.0);
            let x = x as f32;
            let y = y as f32;
            let v = 1.0 + (scale * (x + 0.76 * y)).sin() * (scale * y / 2.0).cos();

            generated_heightmap.push(((u16::MAX / 4) as f32 * v) as u16);
        }
    }
    generated_heightmap
}
// ----------------------------------------------------------------------------
