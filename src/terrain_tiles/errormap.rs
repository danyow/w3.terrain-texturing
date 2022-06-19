// ----------------------------------------------------------------------------
use bevy::{
    math::{uvec2, UVec2},
    prelude::*,
};

use super::generator::TileTriangle;
use super::{TerrainDataView, TerrainTileId, TILE_SIZE};
// ----------------------------------------------------------------------------
type ErrorMapPostprocessingPackage = (Entity, TerrainTileId<TILE_SIZE>, TileHeightErrors);
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct TileHeightErrors {
    errors: Vec<f32>,
}
// ----------------------------------------------------------------------------
pub struct ErrorMapsPostprocessing {
    is_active: bool,
    finished: bool,
    tiles: usize,
    seams: TileHeightErrorSeams,
    queue: Vec<ErrorMapPostprocessingPackage>,
    processed: Vec<ErrorMapPostprocessingPackage>,
}
// ----------------------------------------------------------------------------
/// Holds the table for mapping triangle labels to precalculated triangles.
#[derive(Default)]
pub(super) struct TileTriangleLookup(Vec<PrecalculatedTriangle>);
// ----------------------------------------------------------------------------
impl TileTriangleLookup {
    // ------------------------------------------------------------------------
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    // ------------------------------------------------------------------------
    /// Precalculates all possible triangles for a tile and stores their
    /// coordinates as well as some offsets required for errormap generation in
    /// a lookuptable.
    pub fn generate(&mut self) {
        // all possible triangles and their child triangles are stored in full
        // binary tree:
        //  - lowest level has tilesize * tilesize triangles
        //      - > binary tree depth log(tilesize^2)
        //  - full binary tree has 2^(depth + 1) - 1 elements -> 2 * 2^(depth) -1
        //      -> 2 * 2^log(tilesize^2) -1
        //      -> 2 * tilesize^2 - 1
        let smallest_triangle_count = TILE_SIZE * TILE_SIZE;
        let node_count = smallest_triangle_count * 2 - 1;

        let last_triangle_label = node_count;

        let mut table = vec![PrecalculatedTriangle::default(); 1 + last_triangle_label as usize];

        // no root triangle -> ignore label 1
        for triangle_label in (2..=last_triangle_label).rev() {
            // reconstruct triangle coordinates by splitting from top along path as
            // defined by label and precalculate all offsets into lookup version.
            table[triangle_label as usize] =
                TileTriangle::new_from_path(triangle_label, TILE_SIZE).into();
        }
        self.0 = table;
    }
    // ------------------------------------------------------------------------
    pub fn clear(&mut self) {
        self.0 = Vec::default()
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn get(&self, tirangle_id: u32) -> &PrecalculatedTriangle {
        &self.0[tirangle_id as usize]
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub(super) fn generate_errormap(
    tile_triangles: &TileTriangleLookup,
    heightmap: &TerrainDataView,
) -> TileHeightErrors {
    let mut errors = TileHeightErrors::new();

    // all possible triangles and their child triangles are stored in full
    // binary tree:
    //  - lowest level has tilesize * tilesize triangles
    //      - > binary tree depth log(tilesize^2)
    //  - full binary tree has 2^(depth + 1) - 1 elements -> 2 * 2^(depth) -1
    //      -> 2 * 2^log(tilesize^2) -1
    //      -> 2 * tilesize^2 - 1
    let smallest_triangle_count = TILE_SIZE * TILE_SIZE;
    let node_count = smallest_triangle_count * 2 - 1;

    let last_triangle_label = node_count;
    let smallest_triangle_first_label = last_triangle_label - smallest_triangle_count + 1;

    // no root triangle -> ignore label 1
    for triangle_label in (2..=last_triangle_label).rev() {
        // use pregenerated triangle from lookup table
        let triangle = tile_triangles.get(triangle_label);

        //
        //    .-->  x
        //    |   b
        //    |   |\
        //    V   | \           m: middle point of triangle
        //        R--m          R: right child middle point
        //    y   | /|\         L: left child middle point
        //        |/ | \
        //        c--L--a
        //

        // Note: the tile borders will not match and require postprocessing passes
        // after errormaps for *all* tiles were generated
        let middle_error =
            heightmap.sample_interpolated_height_error(triangle.a(), triangle.b(), triangle.m());

        if triangle_label >= smallest_triangle_first_label {
            // for highest res triangle there is no need to check for any
            // previously set error
            errors.set_unchecked(triangle.middle(), middle_error);
        } else {
            // error in middle point of hypothenuse of left child triangle
            let left_child_error = errors.get_unchecked(triangle.left_middle());
            // error in middle point of hypothenuse of right child triangle
            let right_child_error = errors.get_unchecked(triangle.right_middle());

            errors.update_unchecked(
                triangle.middle(),
                middle_error.max(left_child_error).max(right_child_error),
            );
        }
    }
    errors
}
// ----------------------------------------------------------------------------
pub(super) fn update_errormap(tile_triangles: &TileTriangleLookup, errors: &mut TileHeightErrors) {
    // all possible triangles and their child triangles are stored in full
    // binary tree:
    //  - lowest level has tilesize * tilesize triangles
    //      - > binary tree depth log(tilesize^2)
    //  - full binary tree has 2^(depth + 1) - 1 elements -> 2 * 2^(depth) -1
    //      -> 2 * 2^log(tilesize^2) -1
    //      -> 2 * tilesize^2 - 1
    // let smallest_triangle_count = tilesize * tilesize;
    let smallest_triangle_count = TILE_SIZE * TILE_SIZE;
    let node_count = smallest_triangle_count * 2 - 1;

    let last_triangle_label = node_count;
    let smallest_triangle_first_label = last_triangle_label - smallest_triangle_count + 1;

    // no root triangle -> ignore label 1
    for triangle_label in (2..=smallest_triangle_first_label).rev() {
        // reconstruct triangle coordinates by splitting from top along path as
        // defined by label
        // let triangle = TileTriangle::new_from_path(triangle_label, TILE_SIZE);
        //
        //    .-->  x
        //    |   b
        //    |   |\
        //    V   | \           m: middle point of triangle
        //        R--m          R: right child middle point
        //    y   | /|\         L: left child middle point
        //        |/ | \
        //        c--L--a
        //
        if triangle_label >= smallest_triangle_first_label {
            // no need to update these as only accumulated errors differ in the
            // seam from bordering tiles
        } else {
            let triangle = tile_triangles.get(triangle_label);
            // error in middle point of hypothenuse of left child triangle
            let left_child_error = errors.errors[triangle.left_middle()];
            // error in middle point of hypothenuse of right child triangle
            let right_child_error = errors.errors[triangle.right_middle()];

            errors.update_unchecked(triangle.middle(), left_child_error.max(right_child_error));
        }
    }
}
// ----------------------------------------------------------------------------
impl TileHeightErrors {
    // ------------------------------------------------------------------------
    fn new() -> Self {
        Self {
            errors: vec![0.0; ((TILE_SIZE + 1) * (TILE_SIZE + 1)) as usize],
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub(super) fn coordinate_to_offset(p: UVec2) -> usize {
        (p.y.min(TILE_SIZE) * (TILE_SIZE + 1) + p.x.min(TILE_SIZE)) as usize
    }
    // ------------------------------------------------------------------------
    pub(super) fn get(&self, pos: UVec2) -> f32 {
        self.errors[Self::coordinate_to_offset(pos)]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn get_unchecked(&self, offset: usize) -> f32 {
        self.errors[offset]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn set_unchecked(&mut self, offset: usize, value: f32) {
        self.errors[offset] = value;
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn update_unchecked(&mut self, offset: usize, value: f32) {
        let previous = self.errors[offset];
        if value > previous {
            self.errors[offset] = value;
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn top_seam<const TILE_SIZE_: u32>(&self) -> &[f32] {
        &self.errors[0..=TILE_SIZE_ as usize]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn bottom_seam<const TILE_SIZE_: u32>(&self) -> &[f32] {
        let start = (TILE_SIZE_ * (TILE_SIZE_ + 1)) as usize;
        let end = start + TILE_SIZE_ as usize + 1;
        &self.errors[start..end]
    }
    // ------------------------------------------------------------------------
    fn left_seam<const TILE_SIZE_: u32>(&self) -> impl Iterator<Item = f32> + '_ {
        self.errors.iter().step_by(TILE_SIZE_ as usize + 1).copied()
    }
    // ------------------------------------------------------------------------
    fn right_seam<const TILE_SIZE_: u32>(&self) -> impl Iterator<Item = f32> + '_ {
        self.errors[TILE_SIZE_ as usize..]
            .iter()
            .step_by(TILE_SIZE_ as usize + 1)
            .copied()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// mesh seam optimization
// ----------------------------------------------------------------------------
impl ErrorMapsPostprocessing {
    // ------------------------------------------------------------------------
    pub fn new(map_size: u32, tiles: usize) -> Self {
        Self {
            is_active: false,
            finished: false,
            tiles,
            seams: TileHeightErrorSeams::new(TILE_SIZE as usize, map_size as usize),
            queue: Vec::with_capacity(tiles),
            processed: Vec::with_capacity(tiles),
        }
    }
    // ------------------------------------------------------------------------
    pub fn start(&mut self) {
        self.is_active = true;
        self.finished = false;
        self.merge_seams();
        self.patch_seams();
    }
    // ------------------------------------------------------------------------
    pub fn free_resources(&mut self) {
        self.is_active = false;
        self.finished = false;
        self.seams.reset();
        self.queue = Vec::default();
        self.processed = Vec::default();
    }
    // ------------------------------------------------------------------------
    pub fn add_errormap(
        &mut self,
        entity: Entity,
        tileid: TerrainTileId<TILE_SIZE>,
        errormap: TileHeightErrors,
    ) {
        self.queue.push((entity, tileid, errormap))
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn processing_required(&self) -> bool {
        self.is_active && self.seams.is_dirty
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn next_package(&mut self) -> Option<ErrorMapPostprocessingPackage> {
        self.queue.pop()
    }
    // ------------------------------------------------------------------------
    pub fn append_results(&mut self, results: &mut Vec<ErrorMapPostprocessingPackage>) {
        self.processed.append(results)
    }
    // ------------------------------------------------------------------------
    pub fn drain_results(&mut self) -> impl Iterator<Item = ErrorMapPostprocessingPackage> + '_ {
        assert!(self.finished);
        // Note: after pass is finalized the results are back in input queue
        // because the seam merging was started to check if another pass is
        // required
        self.queue.drain(..)
    }
    // ------------------------------------------------------------------------
    pub fn finalize_pass(&mut self) {
        // seam merge works on input queue
        std::mem::swap(&mut self.queue, &mut self.processed);

        self.merge_seams();

        if self.seams.is_dirty {
            // prepare next pass
            self.patch_seams();
        } else {
            self.finished = true;
        }
    }
    // ------------------------------------------------------------------------
    pub fn progress_info(&self) -> (usize, usize) {
        let tiles_remaining = self
            .tiles
            .saturating_sub(self.seams.mismatched_tiles)
            .min(self.tiles.saturating_sub(self.queue.len()));

        (tiles_remaining, self.tiles)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ErrorMapsPostprocessing {
    // ------------------------------------------------------------------------
    fn merge_seams(&mut self) {
        self.seams.is_dirty = false;
        self.seams.mismatched_tiles = 0;

        for (_, tileid, errormap) in &self.queue {
            self.seams.merge_from::<TILE_SIZE>(*tileid, errormap);
        }
    }
    // ------------------------------------------------------------------------
    fn patch_seams(&mut self) {
        for (_, tileid, errormap) in self.queue.iter_mut() {
            self.seams.patch_error_map::<TILE_SIZE>(*tileid, errormap);
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
struct TileHeightErrorSeams {
    tile_size: usize,
    size: usize,
    horizontal: Vec<f32>,
    vertical: Vec<f32>,

    is_dirty: bool,
    mismatched_tiles: usize,
}
// ----------------------------------------------------------------------------
impl TileHeightErrorSeams {
    // ------------------------------------------------------------------------
    fn new(tile_size: usize, map_size: usize) -> Self {
        let tiles_per_edge = map_size / tile_size;
        Self {
            tile_size,
            // Note: errormaps are tilesize + 1 and have 1px overlapping
            // right/bottom borders
            size: map_size + 1,
            horizontal: vec![0.0; (tiles_per_edge + 1) * (map_size + 1)],
            vertical: vec![0.0; (tiles_per_edge + 1) * (map_size + 1)],

            is_dirty: false,
            mismatched_tiles: 0,
        }
    }
    // ------------------------------------------------------------------------
    fn reset(&mut self) {
        self.horizontal = Vec::default();
        self.vertical = Vec::default();
    }
    // ------------------------------------------------------------------------
    /// extracts left and top seam and merges with previous data (max-test)
    fn merge_from<const TILE_SIZE_: u32>(
        &mut self,
        tileid: TerrainTileId<TILE_SIZE>,
        errormap: &TileHeightErrors,
    ) {
        let mut is_dirty = false;

        // top seam
        let mut offset = self.top_seam_offset(tileid);
        for value in errormap.top_seam::<TILE_SIZE_>() {
            if self.horizontal[offset] < *value {
                self.horizontal[offset] = *value;
                is_dirty = true;
            }
            offset += 1;
        }

        // bottom seam
        let mut offset = self.bottom_seam_offset(tileid);
        for value in errormap.bottom_seam::<TILE_SIZE_>() {
            if self.horizontal[offset] < *value {
                self.horizontal[offset] = *value;
                is_dirty = true;
            }
            offset += 1;
        }

        // left seam
        let mut offset = self.left_seam_offset(tileid);
        for value in errormap.left_seam::<TILE_SIZE_>() {
            if self.vertical[offset] < value {
                self.vertical[offset] = value;
                is_dirty = true;
            }
            offset += 1;
        }

        // right seam
        let mut offset = self.right_seam_offset(tileid);
        for value in errormap.right_seam::<TILE_SIZE_>() {
            if self.vertical[offset] < value {
                self.vertical[offset] = value;
                is_dirty = true;
            }
            offset += 1;
        }

        if is_dirty {
            self.mismatched_tiles += 1;
            self.is_dirty = true;
        }
    }
    // ------------------------------------------------------------------------
    fn patch_error_map<const TILE_SIZE_: u32>(
        &self,
        tileid: TerrainTileId<TILE_SIZE>,
        errormap: &mut TileHeightErrors,
    ) {
        // top
        errormap.errors[0..=self.tile_size].copy_from_slice(self.top_seam(tileid));

        // bottom
        let start = self.tile_size * (self.tile_size + 1);
        let end = start + self.tile_size + 1;
        errormap.errors[start..end].copy_from_slice(self.bottom_seam(tileid));

        // left
        for (old, new) in errormap
            .errors
            .iter_mut()
            .step_by(self.tile_size + 1)
            .zip(self.left_seam(tileid))
        {
            *old = *new;
        }

        // right
        for (old, new) in errormap.errors[self.tile_size..]
            .iter_mut()
            .step_by(self.tile_size + 1)
            .zip(self.right_seam(tileid))
        {
            *old = *new;
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn top_seam(&self, tileid: TerrainTileId<TILE_SIZE>) -> &[f32] {
        let start = self.top_seam_offset(tileid);
        // account for overlapping 1px border!
        &self.horizontal[start..start + self.tile_size + 1]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn bottom_seam(&self, tileid: TerrainTileId<TILE_SIZE>) -> &[f32] {
        let start = self.bottom_seam_offset(tileid);
        // account for overlapping 1px border!
        &self.horizontal[start..start + self.tile_size + 1]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn left_seam(&self, tileid: TerrainTileId<TILE_SIZE>) -> &[f32] {
        let start = self.left_seam_offset(tileid);
        // account for overlapping 1px border!
        &self.vertical[start..start + self.tile_size + 1]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn right_seam(&self, tileid: TerrainTileId<TILE_SIZE>) -> &[f32] {
        let start = self.right_seam_offset(tileid);
        // account for overlapping 1px border!
        &self.vertical[start..start + self.tile_size + 1]
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn top_seam_offset(&self, tileid: TerrainTileId<TILE_SIZE>) -> usize {
        tileid.y() as usize * self.size + tileid.x() as usize * self.tile_size
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn bottom_seam_offset(&self, tileid: TerrainTileId<TILE_SIZE>) -> usize {
        (tileid.y() as usize + 1) * self.size + tileid.x() as usize * self.tile_size
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn left_seam_offset(&self, tileid: TerrainTileId<TILE_SIZE>) -> usize {
        tileid.x() as usize * self.size + tileid.y() as usize * self.tile_size
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn right_seam_offset(&self, tileid: TerrainTileId<TILE_SIZE>) -> usize {
        (tileid.x() as usize + 1) * self.size + tileid.y() as usize * self.tile_size
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
impl TileTriangle {
    // ------------------------------------------------------------------------
    /// reconstructs triangle coordinates by splitting from tilesized quad top
    /// along path as defined by label
    /// Note: right most bit is top level split
    #[inline(always)]
    fn new_from_path(pathlabel: u32, tilesize: u32) -> Self {
        // full triangle binary tree
        //
        //          A           path from root to any node can be described uniquely
        //         / \          by using 0 (1) for left (right) branch.
        //        B   C
        //       /\   /\        for easier subsequent decoding (right shifts) the
        //      D  E F  G       next branch direction is appended to the *left*
        //
        //    node path-label           left-prefixed with 1 by adding 2
        //       ambigous                 unambigous     as number
        //        A   -                     A    1           1
        //        B   0  <- not unique      B   10           2
        //        C   1                     C   11           3
        //        D  00  <- not unique      D  100           4
        //        E  10                     E  110           6
        //        F  01                     F  101           5
        //        G  11                     G  111           7
        //
        //    numbering is unique and can be used as offset-1 to store in array:
        //
        //      array:       A | B | C | D | F | E | G |
        //      idx:         1 | 2   3 | 4   5   6   7 |
        //      depth:       0 | 1   1 | 2   2   2   2 | 3 ... | 4 ...
        //
        //    Important: this path as index stores all nodes of a label in a partition
        //               so iterating from end to front ensures correct propagation
        //               of errors from smaller triangles to larger ones

        // tilesized quad width
        //  ------
        //  |\ TR|
        //  | \  |
        //  |  \ |
        //  | BL\|
        //  ------
        //
        let [mut a, mut b, mut c] = if (pathlabel & 1) == 0 {
            // bottom - left triangle
            [uvec2(tilesize, tilesize), uvec2(0, 0), uvec2(0, tilesize)]
        } else {
            // top right triangle
            [uvec2(0, 0), uvec2(tilesize, tilesize), uvec2(tilesize, 0)]
        };

        let mut label = pathlabel;
        label >>= 1;
        while label > 1 {
            let middle = (a + b) >> 1;

            if (label & 1) == 0 {
                // left sub tree
                b = a;
                a = c;
            } else {
                // right sub tree
                a = b;
                b = c;
            }
            c = middle;
            label >>= 1;
        }

        Self::new(a, b, c)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
/// Same as TileTriangle but contains precalculated offsets and packed
/// coordinates (u32 -> u16) for fast(er) errormap creation. Is not used for
/// actual tile mesh creation!
#[derive(Default, Clone, Copy)]
struct PrecalculatedTriangle {
    a_x: u16,
    a_y: u16,
    b_x: u16,
    b_y: u16,
    m_x: u16,
    m_y: u16,
    left_middle_offset: u32,
    right_middle_offset: u32,
    middle_offset: u32,
} // ----------------------------------------------------------------------------
impl PrecalculatedTriangle {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn is_seam(&self, tile_size: u16) -> bool {
        self.m_x == 0 || self.m_y == 0 || self.m_x == tile_size || self.m_y == tile_size
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn a(&self) -> UVec2 {
        uvec2(self.a_x as u32, self.a_y as u32)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn b(&self) -> UVec2 {
        uvec2(self.b_x as u32, self.b_y as u32)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn m(&self) -> UVec2 {
        uvec2(self.m_x as u32, self.m_y as u32)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn middle(&self) -> usize {
        self.middle_offset as usize
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn left_middle(&self) -> usize {
        self.left_middle_offset as usize
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn right_middle(&self) -> usize {
        self.right_middle_offset as usize
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<TileTriangle> for PrecalculatedTriangle {
    // ------------------------------------------------------------------------
    /// Generate precalculated triangle for errormap generation lookup table.
    #[inline(always)]
    fn from(triangle: TileTriangle) -> Self {
        //
        //    .-->  x
        //    |   b
        //    |   |\
        //    V   | \           m: middle point of triangle
        //        R--m          R: right child middle point
        //    y   | /|\         L: left child middle point
        //        |/ | \
        //        c--L--a
        //
        // required data for errormap generation:
        //  - coordinates for a, b, m
        //  - direct offsets into errormap array for
        //      - left triangle error
        //      - right triangle error
        //      - current triangle
        //
        // Note1: coordinates are tile relative so u16 is enough
        // Note2: errormap offset u32 (4 bytes) instead of usize (8 bytes)
        //
        let m = triangle.m();

        PrecalculatedTriangle {
            a_x: triangle.a().x as u16,
            a_y: triangle.a().y as u16,
            b_x: triangle.b().x as u16,
            b_y: triangle.b().y as u16,
            m_x: m.x as u16,
            m_y: m.y as u16,
            left_middle_offset: TileHeightErrors::coordinate_to_offset(triangle.left_middle())
                as u32,
            right_middle_offset: TileHeightErrors::coordinate_to_offset(triangle.right_middle())
                as u32,
            middle_offset: TileHeightErrors::coordinate_to_offset(m) as u32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// default
// ----------------------------------------------------------------------------
impl Default for ErrorMapsPostprocessing {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
// ----------------------------------------------------------------------------
