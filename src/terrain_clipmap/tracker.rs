// ----------------------------------------------------------------------------
/// Restriction for clipmap level parameter (e.g. if clipmap size is smallish
/// and map size is biggish).
const MAX_SUPPORTED_CLIPMAP_LEVEL: u8 = 8;
// ----------------------------------------------------------------------------
use bevy::math::{uvec2, UVec2, Vec2};

use crate::clipmap::Rectangle;
use crate::terrain_render::{ClipmapInfo, ClipmapLayerInfo};

use super::{ClipmapTracker, LayerRectangle, CLIPMAP_GRANULARITY, CLIPMAP_SIZE};
// ----------------------------------------------------------------------------
impl ClipmapTracker {
    // ------------------------------------------------------------------------
    #[rustfmt::skip]
    pub fn new(data_size: u32, max_level: u8) -> Self {
        assert!(data_size.is_power_of_two(), "only power of two for data size supported");
        assert!(data_size >= CLIPMAP_SIZE, "data size must be >= CLIPMAP_SIZE");
        assert!(max_level > 0, "max level must be > 0");

        let layer_rectangles = Self::generate_layers(data_size, max_level);

        Self {
            layer_rectangles,
            world_offset: Vec2::ZERO,
            world_resolution: 1.0,
            data_size,
            last_pos: Vec2::ZERO,

            forced_update: false,
        }
    }
    // ------------------------------------------------------------------------
    fn generate_layers(data_size: u32, max_level: u8) -> Vec<LayerRectangle> {
        // calculate max possible levels for data size
        // f32 workaround until feature 'int_log' is stable
        let max_level_by_size = 1 + ((data_size / CLIPMAP_SIZE) as f32).log2() as u8;

        let level = max_level
            .min(max_level_by_size)
            .min(MAX_SUPPORTED_CLIPMAP_LEVEL);

        // Note: layer rectangle size represent the *covered* data size of the
        // clipmap window in original resolution.

        // prepare rectangles with appropriate sizes
        if level < 2 {
            // special case just generate two layers: full res and full data size
            vec![
                LayerRectangle::new(UVec2::ZERO, CLIPMAP_SIZE),
                LayerRectangle::new(UVec2::ZERO, data_size),
            ]
        } else {
            // interpolate n = level steps between 0..(max_level_by_size - 1)
            // to get equally distanced exponents for rectangle size calculation:
            //   size = 2^exp * CLIPMAP_SIZE
            //
            // this way first level (full res) always will be
            //      2^0 = 1 * CLIPMAP_SIZE = CLIPMAP_SIZE
            //
            // and last level (max downscaled covering complete data):
            //      2^(max_level_by_size - 1) * CLIPMAP_SIZE
            //    = 2^(log2(data_size / CLIPMAP_SIZE)) * CLIPMAP_SIZE
            //    = data_size
            //
            (0..level as u32)
                .map(|i| {
                    let exp = i as f32 / (level - 1) as f32 * (max_level_by_size - 1) as f32;
                    let size = CLIPMAP_SIZE * 2u32.pow(exp.round() as u32);
                    LayerRectangle::new(UVec2::ZERO, size)
                })
                .collect::<Vec<_>>()
        }
    }
    // ------------------------------------------------------------------------
    /// Clipmap layer backing data sizes. Every clipmap layer datastore contains
    /// the full dataview reduced to a lower resolution: size of layer 0 data is
    /// always original full resolution size and size of last layer is always
    /// CLIPMAP_SIZE.
    pub fn data_view_sizes(&self) -> Vec<u32> {
        self.layer_rectangles
            .iter()
            // (internal) layer rectangle size represent the *covered* data size
            // of the clipmap window in original resolution:
            //  layer 0 covers CLIPMAP_SIZE (since it's a full res view)
            //  layer n covers complete data (since it's reduced to CLIPMAP_SIZE)
            // -> full data size / rectangle size * CLIPMAP_SIZE
            .map(|l| self.data_size / l.rectangle.size.x * CLIPMAP_SIZE)
            .collect()
    }
    // ------------------------------------------------------------------------
    pub fn rectangles(&self) -> Vec<Rectangle> {
        self.layer_rectangles
            .iter()
            .map(|l| l.rectangle.clone())
            .collect()
    }
    // ------------------------------------------------------------------------
    /// mapping between world coordinates and clipmap unsigned texture coordinates
    /// clipmap.xy = (world.xy - map_offset.xy) / map_resolution
    pub fn set_position_mapping(mut self, world_resolution: f32, world_offset: Vec2) -> Self {
        self.world_offset = world_offset;
        self.world_resolution = world_resolution;
        self
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn world_pos_to_map_pos(&self, pos: Vec2) -> UVec2 {
        ((pos - self.world_offset) / self.world_resolution)
            .as_uvec2()
            // clamp to data size
            .min(uvec2(self.data_size-1, self.data_size-1))
    }
    // ------------------------------------------------------------------------
    pub fn force_update(&mut self) {
        self.forced_update = true;
    }
    // ------------------------------------------------------------------------
    /// skips update if new position did not change significantly from last
    /// run check
    pub fn lazy_update(&mut self, pos: Vec2) -> bool {
        if self.forced_update || self.last_pos.distance(pos) > (CLIPMAP_GRANULARITY / 4) as f32 {
            self.update(pos)
        } else {
            false
        }
    }
    // ------------------------------------------------------------------------
    pub fn update(&mut self, pos: Vec2) -> bool {
        // granularity ensures the position is always snapped to same grid positions
        let granularity = CLIPMAP_GRANULARITY;
        let data_max = uvec2(self.data_size / granularity, self.data_size / granularity);

        // required for lazy updates (== check updates only if camera moved at
        // least some distance)
        self.last_pos = pos;

        // remap world pos to clipmap UVec
        let pos = self.world_pos_to_map_pos(pos);

        // update only layer that actually change (check with past rectangles)
        let mut changed = false;

        for r in self.layer_rectangles.iter_mut() {
            // make sure the full rectangle stays inside map even if camera moves
            // out of map boundaries
            let half_rectangle = r.rectangle.size / granularity / 2;
            let clip_pos = (pos / granularity)
                .max(half_rectangle)
                .min(data_max - half_rectangle);
            let pos_min = (clip_pos - half_rectangle) * granularity;

            if r.rectangle.pos != pos_min {
                r.rectangle.pos = pos_min;
                r.changed = true;
                changed = true;
            } else {
                r.changed = false;
            }

            r.changed |= self.forced_update;
            changed |= self.forced_update;
        }

        self.forced_update = false;
        changed
    }
    // ------------------------------------------------------------------------
    /// Maps rectangle defined by min/max world coordinates to layer with highest
    /// resolution that spans complete rectangle.
    pub fn map_to_level(&self, min: Vec2, max: Vec2) -> u8 {
        // assuming big terrain many tiles are only covered by lower layers
        // -> iterate from last lowest res layer to highes res
        // default is max layer
        let mut result = self.layer_rectangles.len() as u8 - 1;
        let min = self.world_pos_to_map_pos(min);
        let max = self.world_pos_to_map_pos(max);
        for (i, layer) in self.layer_rectangles.iter().enumerate().rev().skip(1) {
            if layer.rectangle.covers(min, max) {
                result = i as u8;
            } else {
                return result;
            }
        }
        result
    }
    // ------------------------------------------------------------------------
    pub fn layers(&self) -> impl Iterator<Item = (usize, &LayerRectangle)> {
        self.layer_rectangles.iter().enumerate()
    }
    // ------------------------------------------------------------------------
    pub fn level_count(&self) -> u8 {
        self.layer_rectangles.len() as u8
    }
    // ------------------------------------------------------------------------
    pub fn datasource_size(&self) -> u32 {
        self.data_size
    }
    // ------------------------------------------------------------------------
    pub fn info(&self) -> ClipmapInfo {
        ClipmapInfo::new(
            self.world_offset,
            self.world_resolution,
            CLIPMAP_SIZE,
            self.layer_rectangles
                .iter()
                .map(|l| ClipmapLayerInfo::new(l.rectangle(), CLIPMAP_SIZE))
                .collect(),
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
impl LayerRectangle {
    // ------------------------------------------------------------------------
    fn new(pos: UVec2, size: u32) -> Self {
        Self {
            rectangle: Rectangle {
                pos,
                size: uvec2(size, size),
            },
            changed: false,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Rectangle {
    // ------------------------------------------------------------------------
    /// [min..max[
    #[inline(always)]
    fn covers(&self, min: UVec2, max: UVec2) -> bool {
        self.pos.x <= min.x
            && self.pos.y <= min.y
            && max.x <= self.pos.x + self.size.x
            && max.y <= self.pos.y + self.size.y
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
