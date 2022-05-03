// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
};

use crate::texturearray::{TextureArray, TextureArrayBuilder};
// ----------------------------------------------------------------------------
#[derive(Default, Clone, Debug)]
pub struct Rectangle {
    pub pos: UVec2,
    pub size: UVec2,
}
// ----------------------------------------------------------------------------
pub trait ClipmapData: Default {
    type DataType: Copy;
    // ------------------------------------------------------------------------
    /// Datapoint size to be used for calculating slice length, e.g.
    ///     4 for RGBA 8 bit datapoints as underlying data type is u8
    ///     1 for 16 bit datapoint if data type is u16
    fn datapoint_size(&self) -> u32;
    // ------------------------------------------------------------------------
    fn texture_format(&self) -> TextureFormat;
    // ------------------------------------------------------------------------
    fn wrap_as_image(&self, size: u32, data: Vec<Self::DataType>) -> image::DynamicImage;
    // ------------------------------------------------------------------------
    fn size(&self) -> u32;
    // ------------------------------------------------------------------------
    fn as_slice(&self) -> &[Self::DataType];
    fn as_slice_mut(&mut self) -> &mut [Self::DataType];
    // ------------------------------------------------------------------------
    fn downscale(
        &self,
        src: &[Self::DataType],
        src_size: usize,
        src_x: usize,
        src_y: usize,
        src_roi_size: usize,
        target_size: usize,
    ) -> Vec<Self::DataType>;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub struct Clipmap<const CLIPMAP_SIZE: u32, D: ClipmapData> {
    /// debug name
    label: String,
    /// full res source data
    data: D,
    /// full res data size (width == height)
    data_size: u32,
    /// source data sizes for every layer (width == height). layer 0 is full res.
    layer_sizes: Vec<u32>,
    /// handle of target texture array
    array: Handle<TextureArray>,
    /// optional cache for pregenerated reduced levels to speed up updates.
    /// Note: layer 0 is first downscaled level as full resolution can be
    /// accessed directly via self.data.as_slice().
    cache: Vec<Vec<D::DataType>>,
}
// ----------------------------------------------------------------------------
impl<const CLIPMAP_SIZE: u32, D: ClipmapData> Clipmap<CLIPMAP_SIZE, D> {
    // ------------------------------------------------------------------------
    pub fn label(&self) -> &str {
        &self.label
    }
    // ------------------------------------------------------------------------
    pub fn array(&self) -> &Handle<TextureArray> {
        &self.array
    }
    // ------------------------------------------------------------------------
    pub fn update_layer(
        &self,
        level: u8,
        rectangle: &Rectangle,
        texture_arrays: &mut Assets<TextureArray>,
    ) {
        let new_data = self.generate_layer(level as usize, rectangle);
        texture_arrays
            .get_mut(&self.array)
            .expect("clipmap texture array missing")
            .update_slot(level, new_data);
    }
    // ------------------------------------------------------------------------
    pub fn fullres_data_slice(&self) -> &[D::DataType] {
        self.data.as_slice()
    }
    // ------------------------------------------------------------------------
    pub fn extract_fullres(&self, rectangle: &Rectangle) -> Vec<D::DataType> {
        self.extract(self.data.as_slice(), self.data_size, rectangle)
    }
    // ------------------------------------------------------------------------
    pub fn update_fullres(&mut self, rectangle: &Rectangle, new_data: &[D::DataType]) {
        assert!(rectangle.size.x >= 1);
        assert!(rectangle.size.y >= 1);
        assert!(rectangle.pos.x + rectangle.size.x <= self.data_size);
        assert!(rectangle.pos.y + rectangle.size.y <= self.data_size);
        assert!(rectangle.size.x * rectangle.size.y == new_data.len() as u32);

        let datapoint_size = self.data.datapoint_size() as usize;
        let target_dataline_size = datapoint_size * self.data_size as usize;
        let src_dataline_size = datapoint_size * rectangle.size.x as usize;

        let mut target_offset =
            datapoint_size * (rectangle.pos.y * self.data_size + rectangle.pos.x) as usize;
        let mut src_offset = 0;

        let target_data = self.data.as_slice_mut();
        for _ in 0..rectangle.size.y {
            target_data[target_offset..target_offset + src_dataline_size]
                .copy_from_slice(&new_data[src_offset..src_offset + src_dataline_size]);

            target_offset += target_dataline_size;
            src_offset += src_dataline_size;
        }
    }
    // ------------------------------------------------------------------------
    pub fn disable_cache(&mut self) {
        if !self.cache.is_empty() {
            self.cache = Vec::default();
        }
    }
    // ------------------------------------------------------------------------
    pub fn enable_cache(&mut self) {
        if self.cache.is_empty() && self.data_size > 0 {
            self.generate_cache();
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Helper clipmap builder to hide cache generation and texture array init.
// ----------------------------------------------------------------------------
pub struct ClipmapBuilder<const CLIPMAP_SIZE: u32, D: ClipmapData> {
    clipmap: Clipmap<CLIPMAP_SIZE, D>,
    /// texture format of used data (required to setup texture array)
    format: TextureFormat,
    enable_cache: bool,
}
// ----------------------------------------------------------------------------
impl<const CLIPMAP_SIZE: u32, D: ClipmapData> ClipmapBuilder<CLIPMAP_SIZE, D> {
    // ------------------------------------------------------------------------
    pub fn new(label: &str, clipmap_data: D, full_size: u32, layer_sizes: Vec<u32>) -> Self {
        #[rustfmt::skip]
        assert!(full_size.is_power_of_two(), "{label}: only power of two for data size supported");
        #[rustfmt::skip]
        assert!(full_size >= CLIPMAP_SIZE, "{label}: data size [{full_size}] must be >= CLIPMAP_SIZE");
        #[rustfmt::skip]
        assert!(full_size == clipmap_data.size(), "{label}: {full_size} != clipmap data size {}", clipmap_data.size());

        // ATM for performance reasons: make sure layer res can be divided
        // without remainder by subsequent layer, e.g. 4096 / 1024

        // this is also important for clipmap that is not downscaled with a
        // filter (e.g. texture control). TODO: is it really required?
        assert!(!layer_sizes.is_empty());
        assert!(layer_sizes[0] == full_size);
        let mut prev_size = full_size;
        for size in layer_sizes.iter().copied().skip(1) {
            assert!(prev_size % size == 0,
                "{label}: clipmap layer size {prev_size} must be divideable by next level size {size}");
            prev_size = size;
        }

        let format = clipmap_data.texture_format();

        let clipmap = Clipmap {
            label: label.to_string(),
            data: clipmap_data,
            data_size: full_size,
            layer_sizes,
            array: Handle::default(),
            cache: Vec::default(),
        };

        Self {
            clipmap,
            format,
            enable_cache: false,
        }
    }
    // ------------------------------------------------------------------------
    pub fn enable_cache(mut self, enable: bool) -> Self {
        self.enable_cache = enable;
        self
    }
    // ------------------------------------------------------------------------
    pub fn build(
        self,
        rectangles: Vec<Rectangle>,
        texture_arrays: &mut Assets<TextureArray>,
    ) -> Clipmap<CLIPMAP_SIZE, D> {
        // WORKAROUND to enable usage of data_size == clipmap_size usecase (e.g. debugging)
        // texture_array requires at least two entries
        let mut rectangles = rectangles;
        if rectangles.len() == 1 {
            rectangles.push(rectangles[0].clone());
        }

        let mut clipmap = self.clipmap;
        // clipmap.init
        if self.enable_cache {
            clipmap.generate_cache();
        }

        // no mipmaps for clipmap since only the highest res will be used
        let mut builder =
            TextureArrayBuilder::new(CLIPMAP_SIZE, rectangles.len() as u32, self.format, None);

        // generate initial clipmap level to setup texture array
        for (level, layer_rectangle) in rectangles.iter().enumerate() {
            builder.add_texture(clipmap.generate_layer(level, layer_rectangle));
        }

        clipmap.array =
            texture_arrays.add(builder.add_usage(TextureUsages::STORAGE_BINDING).build());
        clipmap
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// private impl
// ----------------------------------------------------------------------------
impl<const CLIPMAP_SIZE: u32, D: ClipmapData> Clipmap<CLIPMAP_SIZE, D> {
    // ------------------------------------------------------------------------
    fn extract(
        &self,
        src: &[D::DataType],
        src_size: u32,
        rectangle: &Rectangle,
    ) -> Vec<D::DataType> {
        assert!(rectangle.size.x >= 1);
        assert!(rectangle.size.y >= 1);
        assert!(rectangle.pos.x + rectangle.size.x <= self.data_size);
        assert!(rectangle.pos.y + rectangle.size.y <= self.data_size);

        let datapoint_size = self.data.datapoint_size() as usize;
        let src_dataline_size = datapoint_size * src_size as usize;
        let target_dataline_size = datapoint_size * rectangle.size.x as usize;

        let mut result =
            Vec::with_capacity(datapoint_size * (rectangle.size.y * rectangle.size.x) as usize);
        let mut offset = datapoint_size * (rectangle.pos.y * src_size + rectangle.pos.x) as usize;

        for _ in 0..rectangle.size.y {
            result.extend_from_slice(&src[offset..offset + target_dataline_size]);
            offset += src_dataline_size;
        }

        result
    }
    // ------------------------------------------------------------------------
    fn generate_layer(&self, level: usize, rectangle: &Rectangle) -> image::DynamicImage {
        if level == 0 {
            self.data.wrap_as_image(
                CLIPMAP_SIZE,
                self.extract(self.data.as_slice(), self.data_size, rectangle),
            )
        } else if let Some(downscaled) = self.cache.get(level - 1) {
            // use pregenerated downscaled layer to extract rectangle
            // adjust full res rectangle based on clipmap level
            let level_size = self.layer_sizes[level as usize];
            let scale = self.data_size / level_size;

            self.data.wrap_as_image(
                CLIPMAP_SIZE,
                self.extract(
                    downscaled,
                    level_size,
                    &Rectangle {
                        pos: rectangle.pos / scale,
                        size: rectangle.size / scale,
                    },
                ),
            )
        } else {
            self.data.wrap_as_image(
                CLIPMAP_SIZE,
                self.data.downscale(
                    self.data.as_slice(),
                    self.data_size as usize,
                    rectangle.pos.x as usize,
                    rectangle.pos.y as usize,
                    rectangle.size.x as usize,
                    CLIPMAP_SIZE as usize,
                ),
            )
        }
    }
    // ------------------------------------------------------------------------
    fn generate_cache(&mut self) {
        debug!("generating {} cache...", self.label);
        let mut cache = Vec::with_capacity(self.layer_sizes.len() - 1);

        // first level is full res and can be skipped
        for level_size in self.layer_sizes.iter().copied().skip(1) {
            let level_size = level_size as usize;

            // if cache is deactivated during painting the clipmap levels are
            // always generated from highest res. to exactly match the result
            // (and to support seamless enabling/disabling cache) every cached
            // clipmap level must also be generated from full res data.
            let cache_data = self.data.downscale(
                self.data.as_slice(),
                self.data_size as usize,
                0,
                0,
                self.data_size as usize,
                level_size,
            );

            cache.push(cache_data);
        }

        self.cache = cache;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<const CLIPMAP_SIZE: u32, D: ClipmapData> Default for Clipmap<CLIPMAP_SIZE, D> {
    fn default() -> Self {
        Self {
            label: "uninitialized".into(),
            data: D::default(),
            data_size: 0,
            layer_sizes: Vec::default(),
            array: Handle::default(),
            cache: Vec::default(),
        }
    }
}
// ----------------------------------------------------------------------------
