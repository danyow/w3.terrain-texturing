// ----------------------------------------------------------------------------
// Based on "Right-Triangulated Irregular Networks" [1] by Will Evans et al. (1997)
// and an implementation of it: "MARTINI: Real-Time RTIN Terrain Mesh" [2]
//
// This implementation works on a 2^k fullsized heightmap but generates a mesh
// for a smaller tile (e.g 64x64 or 256x256) at the appropriate position. The
// mesh is generated from precalculated per tile errormap.
//
// Tile seams are problematic because errormaps accumulate from different points
// in the tile (see below) and thus have different *accumulated* error values at
// the same positions in an overlapping seams resulting in different triangles.
// -> For now a brute force approach of using a full res triangulation for the
// seam is implemented.
// TODO find a better way to make proper seams.
//
// [1] https://www.cs.ubc.ca/~will/papers/rtin.pdf
// [2] https://observablehq.com/@mourner/martin-real-time-rtin-terrain-mesh
//
// ----------------------------------------------------------------------------
//
// basic idea of mesh creation from errormap:
//  - 2^k + 1 tile is subdivided into two right-angled triangles (left & right):
//
//    .-->  x
//    |   a-----b
//    |   |\    |
//    V   | \   |
//        |  x  |
//    y   |   \ |
//        |    \|
//        d-----c
//
//  - height for vertices is sampled from underlying heightmap
//    Note: vertex (x,y) coordinates are integers and correspond to
//          heightmap pixels.
//    Note: 2^k + 1 tilesize makes sure the middlepoint will be mapped to a
//          heightmap pixel.
//  - interpolate height in the middle (x) of the triangle base (hypothenuse)
//    and compare to ground truth from heightmap
//  - if (absolute) difference is greater than an error threshold, split both
//    triangles into two new rightangle triangles respectively.
//    Note: x will be the "new" rightrangle vertex of all new triangles in
//          this iteration -> no T-Section on hypothenuse in any non border
//          triangle
//  - repeat recursively for new triangles until error threshold condition
//    is met (or lowest level)
//
// basic idea of errormap creation:
//  - accumulate *max* error from subtriangles into parent triangle error at
//    middle of hypothenuses (vertex x)
//  - at deepest recursion a, b, c, d are neighbors in the heightmap and the
//    middlepoint error is defined as zero.
//  - it's important to propagate errors one level up only after the complete
//    level is calculated, e.g. a depth-first binary tree run does not work
//    because a specific errormap value may be updated by neighboring
//    triangle in other parent subtree, too
//  - splitting triangle in two subtriangles creates a full binary tree and
//    the path of a triangle (left, right sequence to tree node) can be used
//    as unique triangle label.
//  - full tree can be stored as array:
//
//          A           path from root to any node can be described uniquely
//         / \          by using 0 (1) for left (right) branch.
//        B   C
//       /\   /\        for easier subsequent decoding (right shifts) the
//      D  E F  G       next branch direction bit is appended to the *left*
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
//               so iterating from end to front ensures correct processing
//               order for propagation of errors from smaller triangles to
//               larger ones
//
//  - triangle label is used to reconstruct the triangle coordinates:
//    since errormap is a preprocessing step that needs to be done once, the
//    coordinates for a traingle are calculated on the fly: basically by
//    recursively splitting root triangle and picking next triangle as
//    indicated by the triangle path-label
//    Note: vertices are shared among multiple triangles so coordinates
//          could be precalculated once into LUT at some memory expense
//
// basic idea of triangle error mapping / storing:
//  - middle of hypothenuse (ab) in any 2D right angle triangle is
//      a + (b - a) / 2  == a/2 + b/2 == (a + b) >> 1
//    >> which will always be a heightmap pixel (because fo 2^k + 1 gridsize) <<
//    >> exception: lowest level where (b-a) < 1 but error is defined as 0
//  - thus coordinates of middlepoint can be used as (x,y) address in a
//    storage array sized (2^k + 1) * (2^k + 1). triangle middle point is
//    also used for lookup
//
// ----------------------------------------------------------------------------
use bevy::{
    math::{uvec2, uvec3, UVec2, UVec3},
    prelude::*,
    render::{mesh::Indices, render_resource::internal::bytemuck::cast},
    utils::HashMap,
};

use super::{TerrainDataView, TerrainMesh, TerrainMeshVertexData, TerrainTileId, TILE_SIZE};
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
                TileTriangle::new_from_path(triangle_label, TILE_SIZE).cook();
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
pub(super) fn generate_tilemesh(
    tile_id: TerrainTileId<TILE_SIZE>,
    map_resolution: f32,
    base_height: f32,
    error_threshold: f32,
    terraindata_view: TerrainDataView,
    triangle_errors: &TileHeightErrors,
    include_wireframe_info: bool,
) -> TerrainMesh {
    if include_wireframe_info {
        generate_mesh(
            error_threshold,
            triangle_errors,
            WireframedTileMeshBuilder::new(tile_id, map_resolution, base_height, terraindata_view),
        )
    } else {
        generate_mesh(
            error_threshold,
            triangle_errors,
            TileMeshBuilder::new(tile_id, map_resolution, base_height, terraindata_view),
        )
    }
}
// ----------------------------------------------------------------------------
fn generate_mesh(
    error_threshold: f32,
    triangle_errors: &TileHeightErrors,
    mut mesh_builder: impl MeshBuilder,
) -> TerrainMesh {
    // top tile triangles are always added
    process_triangle(
        TileTriangle::root_left_bottom(),
        error_threshold,
        triangle_errors,
        &mut mesh_builder,
    );
    process_triangle(
        TileTriangle::root_right_upper(),
        error_threshold,
        triangle_errors,
        &mut mesh_builder,
    );

    mesh_builder.build()
}
// ----------------------------------------------------------------------------
fn process_triangle(
    triangle: TileTriangle,
    error_threshold: f32,
    error_map: &TileHeightErrors,
    mesh_builder: &mut impl MeshBuilder,
) {
    // calculate middle point which is used as lookup address in error map
    if triangle.can_be_split() && error_map.get(triangle.m()) > error_threshold {
        process_triangle(
            triangle.split_left(),
            error_threshold,
            error_map,
            mesh_builder,
        );
        process_triangle(
            triangle.split_right(),
            error_threshold,
            error_map,
            mesh_builder,
        );
    } else {
        mesh_builder.add_triangle(triangle);
    }
}
// ----------------------------------------------------------------------------
/// Right-angled triangle with counter clockwise vertices [a, b, c] where c is
/// vertex of right angle (opposite to hypothenuse).
struct TileTriangle {
    a: UVec2,
    b: UVec2,
    /// vertex opposite to hypothenuse
    c: UVec2,
}
// ----------------------------------------------------------------------------
impl TileTriangle {
    // ------------------------------------------------------------------------
    /// returns biggest left bottom triangle of tile quad (root triangle)
    fn root_left_bottom() -> Self {
        // counter clockwise with right angle vertex last
        Self {
            a: uvec2(TILE_SIZE, TILE_SIZE),
            b: uvec2(0, 0),
            c: uvec2(0, TILE_SIZE),
        }
    }
    // ------------------------------------------------------------------------
    /// returns biggest right top triangle of tile quad (root triangle)
    fn root_right_upper() -> Self {
        // counter clockwise with right angle vertex last
        Self {
            a: uvec2(0, 0),
            b: uvec2(TILE_SIZE, TILE_SIZE),
            c: uvec2(TILE_SIZE, 0),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TileTriangle {
    // ------------------------------------------------------------------------
    /// convention: triangle vertices are counter clockwise and right angle
    /// vertex is last
    #[allow(dead_code)]
    fn new(a: UVec2, b: UVec2, right_angle_vertex: UVec2) -> Self {
        Self {
            a,
            b,
            c: right_angle_vertex,
        }
    }
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

        // tilesized quad qith
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

        Self { a, b, c }
    }
    // ------------------------------------------------------------------------
    /// returns coordinates for middle of hypothenuse
    #[inline(always)]
    fn m(&self) -> UVec2 {
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
        // middle point of triangle hypothenuse
        (self.a + self.b) >> 1
    }
    // ------------------------------------------------------------------------
    /// returns left subtriangle of split
    #[inline(always)]
    fn split_left(&self) -> TileTriangle {
        // Note: countre clockwise, right-angle vertex last
        TileTriangle {
            a: self.c,
            b: self.a,
            c: self.m(),
        }
    }
    // ------------------------------------------------------------------------
    /// returns right subtriangle of split
    #[inline(always)]
    fn split_right(&self) -> TileTriangle {
        // Note: countre clockwise, right-angle vertex last
        TileTriangle {
            a: self.b,
            b: self.c,
            c: self.m(),
        }
    }
    // ------------------------------------------------------------------------
    /// returns coordinates for middle of hypothenuse of left subtriangle
    #[inline(always)]
    fn left_middle(&self) -> UVec2 {
        (self.a + self.c) >> 1
    }
    // ------------------------------------------------------------------------
    /// returns coordinates for middle of hypothenuse of right subtriangle
    #[inline(always)]
    fn right_middle(&self) -> UVec2 {
        (self.b + self.c) >> 1
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn can_be_split(&self) -> bool {
        // test if points are neighbors:
        //  yes: triangle cannot be split
        //  no: can be split

        // Note: abs_diff not stable yet  #![feature(int_abs_diff)]
        // if (a.x.abs_diff(c.x) + a.y.abs_diff(c.y)) > 1 { ...
        // #[rustfmt::skip]
        let diff_x = if self.a.x < self.c.x {
            self.c.x - self.a.x
        } else {
            self.a.x - self.c.x
        };
        let diff_y = if self.a.y < self.c.y {
            self.c.y - self.a.y
        } else {
            self.a.y - self.c.y
        };

        diff_x + diff_y > 1
    }
    // ------------------------------------------------------------------------
    /// Generate precalculated triangle for errormap generation lookup table.
    #[inline(always)]
    fn cook(self) -> PrecalculatedTriangle {
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
        let m = self.m();

        PrecalculatedTriangle {
            a_x: self.a.x as u16,
            a_y: self.a.y as u16,
            b_x: self.b.x as u16,
            b_y: self.b.y as u16,
            m_x: m.x as u16,
            m_y: m.y as u16,
            left_middle_offset: TileHeightErrors::coordinate_to_offset(self.left_middle()) as u32,
            right_middle_offset: TileHeightErrors::coordinate_to_offset(self.right_middle()) as u32,
            middle_offset: TileHeightErrors::coordinate_to_offset(m) as u32,
        }
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
}
// ----------------------------------------------------------------------------
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
impl TileHeightErrors {
    // ------------------------------------------------------------------------
    fn new() -> Self {
        Self {
            errors: vec![0.0; ((TILE_SIZE + 1) * (TILE_SIZE + 1)) as usize],
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn coordinate_to_offset(p: UVec2) -> usize {
        (p.y.min(TILE_SIZE) * (TILE_SIZE + 1) + p.x.min(TILE_SIZE)) as usize
    }
    // ------------------------------------------------------------------------
    fn get(&self, pos: UVec2) -> f32 {
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
/// Helper Mesh builder: collects triangles, makes vertices unique, generates
/// appropriate indices and finally the tilemesh data.
struct TileMeshBuilder<'heightmap, 'normalmap> {
    sampling_offset: UVec2,
    resolution: f32,
    base_height: f32,
    terrain_data: TerrainDataView<'heightmap, 'normalmap>,

    known_indices: HashMap<UVec2, u32>,
    indices: Vec<u32>,
    interleaved_buffer: Vec<[f32; 4]>,
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> TileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn new(
        tileid: TerrainTileId<TILE_SIZE>,
        map_resolution: f32,
        base_height: f32,
        terrain_data: TerrainDataView<'heightmap, 'normalmap>,
    ) -> Self {
        // just a guess
        let expected_vertices = (TILE_SIZE * TILE_SIZE) as usize / 8;

        Self {
            sampling_offset: tileid.sampling_offset(),
            resolution: map_resolution,
            base_height,
            terrain_data,

            known_indices: HashMap::with_capacity(expected_vertices),
            indices: Vec::with_capacity(expected_vertices * 2),

            interleaved_buffer: Vec::with_capacity(expected_vertices),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
/// Helper Mesh builder: same as TileMeshBuilder but also adds barycentric info
/// for every vertex which can be used in shader to colorize triangle edges to
/// visualize an additional wireframe on top.
/// Note: this increases the amount of unique vertices (roughly doubles!) and
/// thus increases the buffer size of vertexdata significantly
struct WireframedTileMeshBuilder<'heightmap, 'normalmap> {
    sampling_offset: UVec2,
    resolution: f32,
    base_height: f32,
    terrain_data: TerrainDataView<'heightmap, 'normalmap>,

    known_indices: HashMap<UVec3, u32>,
    indices: Vec<u32>,
    interleaved_buffer: Vec<[f32; 5]>,
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> WireframedTileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn new(
        tileid: TerrainTileId<TILE_SIZE>,
        map_resolution: f32,
        base_height: f32,
        terrain_data: TerrainDataView<'heightmap, 'normalmap>,
    ) -> Self {
        // just a guess
        let expected_vertices = (TILE_SIZE * TILE_SIZE) as usize / 8;

        Self {
            sampling_offset: tileid.sampling_offset(),
            resolution: map_resolution,
            base_height,
            terrain_data,

            known_indices: HashMap::with_capacity(expected_vertices),
            indices: Vec::with_capacity(expected_vertices * 2),

            interleaved_buffer: Vec::with_capacity(expected_vertices),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// helper trait to support different kinds of builders
trait MeshBuilder {
    fn add_triangle(&mut self, triangle: TileTriangle);
    fn build(self) -> TerrainMesh;
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> MeshBuilder for TileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn add_triangle(&mut self, triangle: TileTriangle) {
        for vertex_2d in [triangle.a, triangle.b, triangle.c].iter().copied() {
            if let Some(index) = self.known_indices.get(&uvec2(vertex_2d.x, vertex_2d.y)) {
                self.indices.push(*index);
            } else {
                let next_index = self.interleaved_buffer.len() as u32; //FIXME

                // map heightmap (x,y) coordinates to world terrain coordinates
                let absolute_map_coords = vertex_2d + self.sampling_offset;

                let (height, vertex_normal) = self
                    .terrain_data
                    .sample_height_and_normal(absolute_map_coords);

                // center tile around 0/0
                let new_vertex = [
                    self.resolution * (vertex_2d.x as f32 - (TILE_SIZE / 2) as f32),
                    self.base_height + height,
                    self.resolution * (vertex_2d.y as f32 - (TILE_SIZE / 2) as f32),
                ];

                self.interleaved_buffer.push([
                    new_vertex[0],
                    new_vertex[1],
                    new_vertex[2],
                    // cast packed u32 normal to f32 so the complete array can
                    // be cast as is to &[u8] before upload to gpu
                    cast(vertex_normal)
                ]);

                self.indices.push(next_index);
                self.known_indices
                    .insert(uvec2(vertex_2d.x, vertex_2d.y), next_index);
            }
        }
    }
    // ------------------------------------------------------------------------
    fn build(self) -> TerrainMesh {
        TerrainMesh::new(
            TerrainMeshVertexData::PositionAndNormal(self.interleaved_buffer),
            Indices::U32(self.indices),
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> MeshBuilder for WireframedTileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn add_triangle(&mut self, triangle: TileTriangle) {
        for (i, vertex_2d) in [triangle.a, triangle.b, triangle.c]
            .iter()
            .copied()
            .enumerate()
        {
            if let Some(index) = self
                .known_indices
                .get(&uvec3(i as u32, vertex_2d.x, vertex_2d.y))
            {
                self.indices.push(*index);
            } else {
                let next_index = self.interleaved_buffer.len() as u32;

                // map heightmap (x,y) coordinates to world terrain coordinates
                let absolute_map_coords = vertex_2d + self.sampling_offset;

                let (height, vertex_normal) = self
                    .terrain_data
                    .sample_height_and_normal(absolute_map_coords);

                // center tile around 0/0
                let new_vertex = [
                    self.resolution * (vertex_2d.x as f32 - (TILE_SIZE / 2) as f32),
                    self.base_height + height,
                    self.resolution * (vertex_2d.y as f32 - (TILE_SIZE / 2) as f32),
                ];

                self.interleaved_buffer.push([
                    new_vertex[0],
                    new_vertex[1],
                    new_vertex[2],
                    // cast packed u32 normal to f32 so the complete array can
                    // be cast as is to &[u8] before upload to gpu
                    cast(vertex_normal),

                    // add marker for vertex position in triangle (will be used for barycentric coords)
                    // and cast it to f32 (reason same as above)
                    cast(i as u32),
                ]);

                self.indices.push(next_index);
                self.known_indices
                    .insert(uvec3(i as u32, vertex_2d.x, vertex_2d.y), next_index);
            }
        }
    }
    // ------------------------------------------------------------------------
    fn build(self) -> TerrainMesh {
        TerrainMesh::new(
            TerrainMeshVertexData::WithBarycentricCoordinates(self.interleaved_buffer),
            Indices::U32(self.indices),
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
