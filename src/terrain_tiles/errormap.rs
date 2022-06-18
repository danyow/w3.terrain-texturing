// ----------------------------------------------------------------------------
use bevy::{
    math::{uvec2, UVec2},
    prelude::*,
};

use super::generator::TileTriangle;
use super::{TerrainDataView, TILE_SIZE};
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct TileHeightErrors {
    errors: Vec<f32>,
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

        // temporaery brute force workaround for seams: set max error to ensure
        // full resolution triangulation at seams so it's guaranteed neighboring
        // tiles will be matching
        if triangle.is_seam(TILE_SIZE as u16) {
            // force subdivision
            errors.set_unchecked(triangle.middle(), f32::MAX);
        } else {
            let middle_error = heightmap.sample_interpolated_height_error(
                triangle.a(),
                triangle.b(),
                triangle.m(),
            );

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
    }
    errors
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
}
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
