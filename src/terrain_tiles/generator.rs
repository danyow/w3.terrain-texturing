// ----------------------------------------------------------------------------
// Based on "Right-Triangulated Irregular Networks" [1] by Will Evans et al. (1997)
// and an implementation of it: "MARTINI: Real-Time RTIN Terrain Mesh" [2]
//
// This implementation works on a 2^k fullsized heightmap but generates a mesh
// for a smaller tile (e.g 64x64 or 256x256) at the appropriate position. The
// mesh is generated from a precalculaed errormap.
//
// Tile seams are problematic because errormaps accumulate from different points
// in the tile (see below) and thus have different *accumulated* error values at
// the same positions in an overlapping seams resulting in different triangles.
// -> For now a brute force approach of using a full res triangulation for the
// seam is implemented.
// TODO find a better way to mkae proper seams.
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
    render::mesh::Indices,
    render::render_resource::PrimitiveTopology,
    utils::{AHashExt, HashMap},
};

use super::{TerrainDataView, TerrainMesh, TerrainTileId, TILE_SIZE};
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct TileHeightErrors {
    errors: Vec<f32>,
}
// ----------------------------------------------------------------------------
pub(super) fn generate_errormap(heightmap: &TerrainDataView) -> TileHeightErrors {
    // let mut errors = TileHeightErrors::new(tilesize);
    let mut errors = TileHeightErrors::new();

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
    for triangle_label in (2..=last_triangle_label).rev() {
        // reconstruct triangle coordinates by splitting from top along path as
        // defined by label
        // let triangle = TileTriangle::new(triangle_label, tilesize);
        let triangle = TileTriangle::new_from_path(triangle_label, TILE_SIZE);
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
        if triangle.m().x == 0
            || triangle.m().y == 0
            || triangle.m().x == TILE_SIZE
            || triangle.m().y == TILE_SIZE
        {
            errors.set(triangle.m(), f32::MAX);
        } else {
            let middle_error =
                heightmap.sample_interpolated_height_error(triangle.a, triangle.b, triangle.m());

            if triangle_label >= smallest_triangle_first_label {
                // no need to check for previously set error
                errors.set(triangle.m(), middle_error);
            } else {
                // error in middle point of hypothenuse of left child triangle
                let left_child_error = errors.get(triangle.left_middle());
                // error in middle point of hypothenuse of right child triangle
                let right_child_error = errors.get(triangle.right_middle());

                errors.update(
                    triangle.m(),
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
) -> TileMesh {
    let mut mesh_builder =
        TileMeshBuilder::new(tile_id, map_resolution, base_height, terraindata_view);

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
    mesh_builder: &mut TileMeshBuilder,
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
/// right-angled triangle with counter clockwise vertices [a, b, c] where c is
/// vertex of right angle (opposite to hypothenuse)
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
    fn coordinate_to_offset(&self, p: UVec2) -> usize {
        (p.y.min(TILE_SIZE) * (TILE_SIZE + 1) + p.x.min(TILE_SIZE)) as usize
    }
    // ------------------------------------------------------------------------
    fn get(&self, pos: UVec2) -> f32 {
        self.errors[self.coordinate_to_offset(pos)]
    }
    // ------------------------------------------------------------------------
    fn set(&mut self, pos: UVec2, value: f32) {
        let offset = self.coordinate_to_offset(pos);
        self.errors[offset] = value;
    }
    // ------------------------------------------------------------------------
    fn update(&mut self, pos: UVec2, value: f32) -> f32 {
        let offset = self.coordinate_to_offset(pos);
        let previous = self.errors[offset];
        if value > previous {
            self.errors[offset] = value;
            value
        } else {
            previous
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

    known_indices: HashMap<UVec3, u32>,
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    barycentric: Vec<[f32; 2]>,
    indices: Vec<u32>,
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
            vertices: Vec::with_capacity(expected_vertices),
            normals: Vec::with_capacity(expected_vertices),
            barycentric: Vec::with_capacity(expected_vertices),
            indices: Vec::with_capacity(expected_vertices * 2),
        }
    }
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
                let next_index = self.vertices.len() as u32;

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

                let vertex_normal = [vertex_normal[0], vertex_normal[1], vertex_normal[2]];

                self.vertices.push(new_vertex);
                self.normals.push(vertex_normal);
                self.indices.push(next_index);

                match i {
                    0 => self.barycentric.push([1.0, 0.0]),
                    1 => self.barycentric.push([0.0, 1.0]),
                    _ => self.barycentric.push([0.0, 0.0]),
                }

                self.known_indices
                    .insert(uvec3(i as u32, vertex_2d.x, vertex_2d.y), next_index);
            }
        }
    }
    // ------------------------------------------------------------------------
    fn build(self) -> TileMesh {
        TileMesh {
            vertices: self.vertices,
            normals: self.normals,
            indices: self.indices,
            barycentric: Some(self.barycentric),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub(super) struct TileMesh {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,

    barycentric: Option<Vec<[f32; 2]>>,
}
// ----------------------------------------------------------------------------
impl TileMesh {
    // ------------------------------------------------------------------------
    pub(super) fn mesh(mut self) -> TerrainMesh {
        let mut mesh = TerrainMesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(TerrainMesh::ATTRIBUTE_POSITION, self.vertices);
        mesh.set_attribute(TerrainMesh::ATTRIBUTE_NORMAL, self.normals);
        if let Some(coords) = self.barycentric.take() {
            mesh.set_attribute(TerrainMesh::ATTRIBUTE_UV_0, coords);
        }
        mesh.set_indices(Some(Indices::U32(self.indices)));
        mesh
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
