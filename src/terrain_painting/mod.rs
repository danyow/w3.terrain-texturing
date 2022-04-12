// ----------------------------------------------------------------------------
use bevy::math::{uvec2, vec2};
use bevy::{ecs::schedule::StateData, prelude::*};

use crate::config::TerrainConfig;
use crate::terrain_clipmap::{ClipmapTracker, TextureControlClipmap};
use crate::terrain_material::MaterialSlot;

use crate::clipmap::Rectangle;
// ----------------------------------------------------------------------------
pub struct TerrainPaintingPlugin;
// ----------------------------------------------------------------------------
impl TerrainPaintingPlugin {
    // ------------------------------------------------------------------------
    pub fn process_brush_operations<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state).with_system(process_brush_operations)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// painting operations
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct PaintingEvent(BrushPlacement, Vec<PaintCommand>);
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct BrushPlacement {
    pos: Vec2,
    radius: f32,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub struct OverwriteProbability(pub f32);
#[derive(Clone, Copy, Debug)]
pub struct TextureScale(pub u8);
#[derive(Clone, Copy, Debug)]
pub struct SlopeBlendThreshold(pub u8);
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum PaintCommand {
    // textures
    SetOverlayMaterial(MaterialSlot),
    SetBackgroundMaterial(MaterialSlot),
    RandomizedSetOverlayMaterial(OverwriteProbability, MaterialSlot),
    RandomizedSetBackgroundMaterial(OverwriteProbability, MaterialSlot),
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainPaintingPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_event::<PaintingEvent>();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn process_brush_operations(
    config: Res<TerrainConfig>,
    mut paint_events: EventReader<PaintingEvent>,
    mut texture_clipmap: ResMut<TextureControlClipmap>,
    mut clipmap_tracker: ResMut<ClipmapTracker>,
) {
    for PaintingEvent(placement, cmds) in paint_events.iter() {

        let (rectangle, mask) = calculate_region_of_interest(&*config, placement);

        // disabling cache will force clipmaptracker to always use current data
        // for clipmap generation. so it is ok to paint only on highest res data.
        // cache will be (re)enabled everytime freecam is activated to make
        // flying around more smooth
        texture_clipmap.disable_cache();

        let mut data = texture_clipmap.extract_fullres(&rectangle);

        for cmd in cmds {
            match cmd {
                PaintCommand::SetOverlayMaterial(slot) => {
                    paint_overlay_texture(&mask, &mut data, slot)
                }
                PaintCommand::RandomizedSetOverlayMaterial(prob, slot) => {
                    paint_randomized_overlay_texture(&mask, &mut data, slot, *prob);
                }
                PaintCommand::SetBackgroundMaterial(slot) => {
                    paint_background_texture(&mask, &mut data, slot);
                }
                PaintCommand::RandomizedSetBackgroundMaterial(prob, slot) => {
                    paint_randomized_background_texture(&mask, &mut data, slot, *prob);
                }
            }
        }
        // updating full resolution is not enough: the clipmap must also be
        // regenerated and upload to the gpu
        texture_clipmap.update_fullres(&rectangle, &data);
        clipmap_tracker.force_update();
    }
}
// ----------------------------------------------------------------------------
fn calculate_region_of_interest(
    config: &TerrainConfig,
    placement: &BrushPlacement,
) -> (Rectangle, Vec<bool>) {

    // respect resolution of clipmap data which differes from world resolution:
    // map world coordinates/resolution to map coordinates/resolution
    let map_brush_center = config.world_pos_to_map_pos(placement.pos);
    let map_brush_center = vec2(map_brush_center.x as f32, map_brush_center.y as f32);
    let map_radius = (placement.radius / config.resolution()).round();

    let min = config.world_pos_to_map_pos(placement.pos - Vec2::splat(placement.radius));
    let max =
        config.world_pos_to_map_pos(placement.pos + Vec2::splat(placement.radius)) + uvec2(1, 1);

    let size = (max - min).max(uvec2(1, 1));
    let rectangle = Rectangle { pos: min, size };

    // precalculate filter for all pixels not in the brush circle (roughly)
    let mut mask = Vec::with_capacity((size.y * size.x) as usize);
    for y in min.y..min.y + size.y {
        for x in min.x..min.x + size.x {
            let distance = map_brush_center.distance(vec2(x as f32, y as f32));
            //
            mask.push(distance < map_radius);
        }
    }
    // special edge case: prevent completely empty mask (rounding errors)
    if size.x * size.y == 1 {
        mask = vec![true];
    }

    (rectangle, mask)
}
// ----------------------------------------------------------------------------
// painting operations
// ----------------------------------------------------------------------------
// 0..4 overlay texture idx
// 5..9 background textures idx
// 10..15 blend control
//   10..12 slope threshold
//   13..15 UV scale
//
#[inline(always)]
fn paint_overlay_texture(mask: &[bool], data: &mut [u16], slot: &MaterialSlot) {
    // zero is reserved for holes
    let material = **slot as u16 + 1;
    for (d, _) in data.iter_mut().zip(mask.iter()).filter(|(_, m)| **m) {
        *d = (*d & 0b1111_1111_1110_0000) + material;
    }
}
// ----------------------------------------------------------------------------
#[inline(always)]
fn paint_background_texture(mask: &[bool], data: &mut [u16], slot: &MaterialSlot) {
    // zero is reserved for holes
    let material = **slot as u16 + 1;
    for (d, _) in data.iter_mut().zip(mask.iter()).filter(|(_, m)| **m) {
        *d = (*d & 0b1111_1100_0001_1111) + (material << 5);
    }
}
// ----------------------------------------------------------------------------
#[inline(always)]
fn paint_randomized_overlay_texture(
    mask: &[bool],
    data: &mut [u16],
    slot: &MaterialSlot,
    probability: OverwriteProbability,
) {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();

    // zero is reserved for holes
    let material = **slot as u16 + 1;
    for (d, _) in data.iter_mut().zip(mask.iter()).filter(|(_, m)| **m) {
        if rng.gen_bool(*probability as f64) {
            *d = (*d & 0b1111_1111_1110_0000) + material;
        }
    }
}
// ----------------------------------------------------------------------------
#[inline(always)]
fn paint_randomized_background_texture(
    mask: &[bool],
    data: &mut [u16],
    slot: &MaterialSlot,
    probability: OverwriteProbability,
) {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();

    // zero is reserved for holes
    let material = **slot as u16 + 1;
    for (d, _) in data.iter_mut().zip(mask.iter()).filter(|(_, m)| **m) {
        if rng.gen_bool(*probability as f64) {
            *d = (*d & 0b1111_1100_0001_1111) + (material << 5);
        }
    }
}
// ----------------------------------------------------------------------------
// painting event
// ----------------------------------------------------------------------------
impl PaintingEvent {
    // ------------------------------------------------------------------------
    pub fn new(placement: BrushPlacement, cmds: Vec<PaintCommand>) -> Self {
        Self(placement, cmds)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl BrushPlacement {
    // ------------------------------------------------------------------------
    /// pos and radius are interpreted as world position/resolution.
    pub fn new(world_pos: Vec2, radius: f32) -> Self {
        BrushPlacement {
            pos: world_pos,
            radius,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Default impl
// ----------------------------------------------------------------------------
#[allow(clippy::derivable_impls)]
impl Default for TextureScale {
    fn default() -> Self {
        Self(0)
    }
}
// ----------------------------------------------------------------------------
impl Default for SlopeBlendThreshold {
    fn default() -> Self {
        Self(4)
    }
}
// ----------------------------------------------------------------------------
// Deref
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for TextureScale {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl Deref for SlopeBlendThreshold {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl Deref for OverwriteProbability {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------