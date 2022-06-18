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
    render::{mesh::Indices, render_resource::internal::bytemuck::cast},
    utils::HashMap,
};

use super::{
    MeshReduction, TerrainDataView, TerrainMesh, TerrainMeshVertexData, TerrainTileId,
    TileHeightErrors, TILE_SIZE,
};
// ----------------------------------------------------------------------------
/// Right-angled triangle with counter clockwise vertices [a, b, c] where c is
/// vertex of right angle (opposite to hypothenuse).
pub struct TileTriangle {
    a: UVec2,
    b: UVec2,
    /// vertex opposite to hypothenuse
    c: UVec2,
}
// ----------------------------------------------------------------------------
impl TileTriangle {
    // ------------------------------------------------------------------------
    /// convention: triangle vertices are counter clockwise and right angle
    /// vertex is last
    #[inline(always)]
    pub fn new(a: UVec2, b: UVec2, right_angle_vertex: UVec2) -> Self {
        Self {
            a,
            b,
            c: right_angle_vertex,
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn a(&self) -> UVec2 {
        self.a
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn b(&self) -> UVec2 {
        self.b
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn c(&self) -> UVec2 {
        self.c
    }
    // ------------------------------------------------------------------------
    /// returns coordinates for middle of hypothenuse
    #[inline(always)]
    pub fn m(&self) -> UVec2 {
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
    /// returns coordinates for middle of hypothenuse of left subtriangle
    #[inline(always)]
    pub fn left_middle(&self) -> UVec2 {
        (self.a + self.c) >> 1
    }
    // ------------------------------------------------------------------------
    /// returns coordinates for middle of hypothenuse of right subtriangle
    #[inline(always)]
    pub fn right_middle(&self) -> UVec2 {
        (self.b + self.c) >> 1
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn generate_tilemesh(
    tile_id: TerrainTileId<TILE_SIZE>,
    map_resolution: f32,
    base_height: f32,
    error_thresholds: &MeshReduction,
    terraindata_view: TerrainDataView,
    triangle_errors: &TileHeightErrors,
    include_wireframe_info: bool,
    small_index: bool,
) -> TerrainMesh {
    if include_wireframe_info {
        let builder = WireframedTileMeshBuilder::new(
            tile_id,
            map_resolution,
            base_height,
            terraindata_view,
            small_index,
        );

        if small_index {
            generate_mesh_with_small_idx(error_thresholds, triangle_errors, builder)
        } else {
            generate_mesh(error_thresholds, triangle_errors, builder)
        }
    } else {
        let builder = TileMeshBuilder::new(
            tile_id,
            map_resolution,
            base_height,
            terraindata_view,
            small_index,
        );
        if small_index {
            generate_mesh_with_small_idx(error_thresholds, triangle_errors, builder)
        } else {
            generate_mesh(error_thresholds, triangle_errors, builder)
        }
    }
}
// ----------------------------------------------------------------------------
// special case:
//      it's established that the generated mesh will have < u16::MAX vertices
// ----------------------------------------------------------------------------
fn generate_mesh_with_small_idx(
    error_thresholds: &MeshReduction,
    triangle_errors: &TileHeightErrors,
    mut mesh_builder: impl MeshBuilder,
) -> TerrainMesh {
    // top tile triangles are always added
    process_triangle_with_small_idx(
        TileTriangle::root_left_bottom(),
        error_thresholds,
        triangle_errors,
        &mut mesh_builder,
    );
    process_triangle_with_small_idx(
        TileTriangle::root_right_upper(),
        error_thresholds,
        triangle_errors,
        &mut mesh_builder,
    );

    mesh_builder.build()
}
// ----------------------------------------------------------------------------
fn process_triangle_with_small_idx(
    triangle: TileTriangle,
    error_thresholds: &MeshReduction,
    error_map: &TileHeightErrors,
    mesh_builder: &mut impl MeshBuilder,
) {
    // calculate middle point which is used as lookup address in error map
    if triangle.can_be_split()
        && error_map.get(triangle.m()) > error_thresholds.get_error_threshold(&triangle)
    {
        process_triangle_with_small_idx(
            triangle.split_left(),
            error_thresholds,
            error_map,
            mesh_builder,
        );
        process_triangle_with_small_idx(
            triangle.split_right(),
            error_thresholds,
            error_map,
            mesh_builder,
        );
    } else {
        mesh_builder.add_triangle::<true>(triangle);
    }
}
// ----------------------------------------------------------------------------
// conservative case:
//      unknown number of vertices for tile. use u32 indices as default
// ----------------------------------------------------------------------------
fn generate_mesh(
    error_thresholds: &MeshReduction,
    triangle_errors: &TileHeightErrors,
    mut mesh_builder: impl MeshBuilder,
) -> TerrainMesh {
    // top tile triangles are always added
    process_triangle(
        TileTriangle::root_left_bottom(),
        error_thresholds,
        triangle_errors,
        &mut mesh_builder,
    );
    process_triangle(
        TileTriangle::root_right_upper(),
        error_thresholds,
        triangle_errors,
        &mut mesh_builder,
    );

    mesh_builder.build()
}
// ----------------------------------------------------------------------------
fn process_triangle(
    triangle: TileTriangle,
    error_thresholds: &MeshReduction,
    error_map: &TileHeightErrors,
    mesh_builder: &mut impl MeshBuilder,
) {
    // calculate middle point which is used as lookup address in error map
    if triangle.can_be_split()
        && error_map.get(triangle.m()) > error_thresholds.get_error_threshold(&triangle)
    {
        process_triangle(
            triangle.split_left(),
            error_thresholds,
            error_map,
            mesh_builder,
        );
        process_triangle(
            triangle.split_right(),
            error_thresholds,
            error_map,
            mesh_builder,
        );
    } else {
        mesh_builder.add_triangle::<false>(triangle);
    }
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
    indices_u32: Vec<u32>,
    indices_u16: Vec<u16>,
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
        use_small_index: bool,
    ) -> Self {
        let max_vertex_count = (TILE_SIZE * TILE_SIZE) as usize;

        // factor 2 is just a guess
        let (indices_u32, indices_u16) = if use_small_index {
            (Vec::default(), Vec::with_capacity(u16::MAX as usize * 2))
        } else {
            (Vec::with_capacity(max_vertex_count * 2), Vec::default())
        };

        Self {
            sampling_offset: tileid.sampling_offset(),
            resolution: map_resolution,
            base_height,
            terrain_data,

            known_indices: HashMap::with_capacity(max_vertex_count),
            indices_u32,
            indices_u16,

            interleaved_buffer: Vec::with_capacity(max_vertex_count),
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn add_vertex(&mut self, vertex_2d: UVec2) -> u32 {
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
        ]);

        self.known_indices
            .insert(uvec2(vertex_2d.x, vertex_2d.y), next_index);

        next_index
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
    indices_u32: Vec<u32>,
    indices_u16: Vec<u16>,
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
        use_small_index: bool,
    ) -> Self {
        let max_vertex_count = (TILE_SIZE * TILE_SIZE) as usize;

        // factor 2 is just a guess
        let (indices_u32, indices_u16) = if use_small_index {
            (Vec::default(), Vec::with_capacity(u16::MAX as usize * 2))
        } else {
            (Vec::with_capacity(max_vertex_count * 2), Vec::default())
        };

        Self {
            sampling_offset: tileid.sampling_offset(),
            resolution: map_resolution,
            base_height,
            terrain_data,

            known_indices: HashMap::with_capacity(max_vertex_count),
            indices_u32,
            indices_u16,

            interleaved_buffer: Vec::with_capacity(max_vertex_count),
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn add_vertex(&mut self, vertex_2d: UVec2, triangle_corner: u32) -> u32 {
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
            cast(triangle_corner),
        ]);

        self.known_indices
            .insert(uvec3(triangle_corner, vertex_2d.x, vertex_2d.y), next_index);

        next_index
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// helper trait to support different kinds of builders
trait MeshBuilder {
    fn add_triangle<const USE_SMALL_INDEX: bool>(&mut self, triangle: TileTriangle);
    fn build(self) -> TerrainMesh;
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> MeshBuilder for TileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn add_triangle<const USE_SMALL_INDEX: bool>(&mut self, triangle: TileTriangle) {
        for vertex_2d in [triangle.a, triangle.b, triangle.c].iter().copied() {
            if let Some(index) = self.known_indices.get(&uvec2(vertex_2d.x, vertex_2d.y)) {
                if USE_SMALL_INDEX {
                    self.indices_u16.push(*index as u16);
                } else {
                    self.indices_u32.push(*index);
                }
            } else {
                let index = self.add_vertex(vertex_2d);
                if USE_SMALL_INDEX {
                    self.indices_u16.push(index as u16);
                } else {
                    self.indices_u32.push(index);
                }
            }
        }
    }
    // ------------------------------------------------------------------------
    fn build(self) -> TerrainMesh {
        TerrainMesh::new(
            TerrainMeshVertexData::PositionAndNormal(self.interleaved_buffer),
            if self.indices_u16.is_empty() {
                if self.known_indices.len() < u16::MAX as usize {
                    // remap to smaller index
                    Indices::U16(self.indices_u32.iter().copied().map(|i| i as u16).collect())
                } else {
                    Indices::U32(self.indices_u32)
                }
            } else {
                Indices::U16(self.indices_u16)
            },
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normalmap> MeshBuilder for WireframedTileMeshBuilder<'heightmap, 'normalmap> {
    // ------------------------------------------------------------------------
    fn add_triangle<const USE_SMALL_INDEX: bool>(&mut self, triangle: TileTriangle) {
        for (i, vertex_2d) in [triangle.a, triangle.b, triangle.c]
            .iter()
            .copied()
            .enumerate()
        {
            if let Some(index) = self
                .known_indices
                .get(&uvec3(i as u32, vertex_2d.x, vertex_2d.y))
            {
                if USE_SMALL_INDEX {
                    self.indices_u16.push(*index as u16);
                } else {
                    self.indices_u32.push(*index);
                }
            } else {
                let index = self.add_vertex(vertex_2d, i as u32);
                if USE_SMALL_INDEX {
                    self.indices_u16.push(index as u16);
                } else {
                    self.indices_u32.push(index);
                }
            }
        }
    }
    // ------------------------------------------------------------------------
    fn build(self) -> TerrainMesh {
        TerrainMesh::new(
            TerrainMeshVertexData::WithBarycentricCoordinates(self.interleaved_buffer),
            if self.indices_u16.is_empty() {
                if self.known_indices.len() < u16::MAX as usize {
                    // remap to smaller index
                    Indices::U16(self.indices_u32.iter().copied().map(|i| i as u16).collect())
                } else {
                    Indices::U32(self.indices_u32)
                }
            } else {
                Indices::U16(self.indices_u16)
            },
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
