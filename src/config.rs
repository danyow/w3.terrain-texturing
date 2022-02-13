// ----------------------------------------------------------------------------
pub const TILE_SIZE: u32 = 256;
// ----------------------------------------------------------------------------
/// config for texturing maps
#[derive(Clone)]
pub struct TextureMaps {
    // stackvalue := background texture id + overlay textureid + blendcontrol
    background: String,
    overlay: String,
    blendcontrol: String,
}
// ----------------------------------------------------------------------------
/// config for current world/terrain
#[derive(Clone)]
pub struct TerrainConfig {
    name: String,
    /// terrain size in meters
    terrain_size: f32,
    /// pixel size of all maps
    map_size: u32,
    /// precalculated resolution of terrain (terrain_size / map_size)
    resolution: f32,
    /// lowest height of terrain in meters (absolute)
    min_height: f32,
    /// heighest height of terrain in meters (absolute)
    max_height: f32,
    /// path to heightmap
    heightmap: String,
    /// paths to texture and blend control maps
    texturemaps: TextureMaps,
    /// path to tint/pigment/color map
    tintmap: String,
    /// currently assigned materialset info
    materialset: MaterialSetConfig,
}
// ----------------------------------------------------------------------------
use bevy::math::{vec2, Vec2};

use crate::terrain_material::TerrainMaterialParam;
// ----------------------------------------------------------------------------
#[allow(dead_code)]
impl TerrainConfig {
    // ------------------------------------------------------------------------
    pub fn name(&self) -> &str {
        &self.name
    }
    // ------------------------------------------------------------------------
    /// resolution m/px
    pub fn resolution(&self) -> f32 {
        self.resolution
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn terrain_size(&self) -> f32 {
        self.terrain_size
    }
    // ------------------------------------------------------------------------
    pub fn map_size(&self) -> u32 {
        self.map_size
    }
    // ------------------------------------------------------------------------
    pub fn map_offset(&self) -> Vec2 {
        // assumption is: map is centered around origin, 4 tile corners at origin
        let tiles = self.map_size / TILE_SIZE;
        let tile_offset = (tiles / 2) as f32 * TILE_SIZE as f32;

        vec2(-tile_offset, -tile_offset) * self.resolution
    }
    // ------------------------------------------------------------------------
    pub fn min_height(&self) -> f32 {
        self.min_height
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn max_height(&self) -> f32 {
        self.max_height
    }
    // ------------------------------------------------------------------------
    pub fn height_scaling(&self) -> f32 {
        (self.max_height - self.min_height) / u16::MAX as f32
    }
    // ------------------------------------------------------------------------
    pub fn tiles_per_edge(&self) -> u8 {
        (self.map_size / TILE_SIZE) as u8
    }
    // ------------------------------------------------------------------------
    pub fn tile_count(&self) -> usize {
        (self.map_size / TILE_SIZE * self.map_size / TILE_SIZE) as usize
    }
    // ------------------------------------------------------------------------
    pub fn heightmap(&self) -> &str {
        &self.heightmap
    }
    // ------------------------------------------------------------------------
    pub fn tintmap(&self) -> &str {
        &self.tintmap
    }
    // ------------------------------------------------------------------------
    pub fn texturemaps(&self) -> &TextureMaps {
        &self.texturemaps
    }
    // ------------------------------------------------------------------------
    pub fn materialset(&self) -> &MaterialSetConfig {
        &self.materialset
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct MaterialSetConfig {
    /// (full) path to diffuse textures (size and type verified)
    diffuse: Vec<String>,
    /// (full) path to normal textures (size and type verified)
    normal: Vec<String>,
    /// materialsettings
    parameter: Vec<TerrainMaterialParam>,
}
// ----------------------------------------------------------------------------
#[allow(dead_code)]
impl MaterialSetConfig {
    // ------------------------------------------------------------------------
    pub fn texture_size(&self) -> u32 {
        // TODO support other?
        1024
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[allow(dead_code)]
impl TextureMaps {
    // ------------------------------------------------------------------------
    pub fn background(&self) -> &str {
        &self.background
    }
    // ------------------------------------------------------------------------
    pub fn overlay(&self) -> &str {
        &self.overlay
    }
    // ------------------------------------------------------------------------
    pub fn blendcontrol(&self) -> &str {
        &self.blendcontrol
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// dafaults
// ----------------------------------------------------------------------------
impl Default for TerrainConfig {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // let map_size = 512;
        // Self {
        //     name: "Empty Terrain".into(),
        //     terrain_size: map_size as f32 / 2.0,
        //     map_size,
        //     resolution: 0.5,
        //     min_height: 0.0,
        //     max_height: 100.0,
        //     heightmap: String::default(),
        //     texturemaps: TextureMaps {
        //         background: String::default(),
        //         overlay: String::default(),
        //         blendcontrol: String::default(),
        //     },
        //     tintmap: String::default(),
        //     clipmap_levels: 2,
        //     materialset: MaterialSetConfig::default(),
        // }
        Self::prolog_village(1024)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for MaterialSetConfig {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // TODO provide two default materials in assets?
        Self {
            diffuse: vec![String::default(); 31],
            normal: vec![String::default(); 31],
            parameter: vec![TerrainMaterialParam::default(); 31],
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// fmt
// ----------------------------------------------------------------------------
use std::fmt;

impl fmt::Debug for TerrainConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TerrainConfig: {}", self.name())
    }
}
// ----------------------------------------------------------------------------
impl TerrainConfig {
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn prolog_village(size: u32) -> Self {
        let basepath = "_test-data_/terrain/";
        let _hubid = "prolog_village";

        Self {
            name: format!("Prologue ({} x {})", size, size),
            terrain_size: size as f32 / 2.0,
            map_size: size,
            resolution: 0.5,
            min_height: -37.0,
            max_height: 45.0,
            heightmap: format!("{}/test.heightmap.{}x{}.png", basepath, size, size),
            texturemaps: TextureMaps {
                background: format!("{}/test.bkgrnd.{}x{}.png", basepath, size, size),
                overlay: format!("{}/test.overlay.{}x{}.png", basepath, size, size),
                blendcontrol: format!("{}/test.blendcontrol.{}x{}.png", basepath, size, size),
            },
            tintmap: format!("{}/test.tint.{}x{}.png", basepath, size, size),
            materialset: MaterialSetConfig::prolog_village(),
        }
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn kaer_morhen() -> Self {
        let basepath = "_test-data_/terrain/";
        let _hubid = "kaer_morhen";
        let size = 16384;
        Self {
            name: "Kaer Morhen (16384 x 16384)".into(),
            terrain_size: size as f32 / 2.0,
            map_size: size,
            resolution: 0.5,
            min_height: -118.0,
            max_height: 1682.0,
            heightmap: format!("{}/test.heightmap.{}x{}.png", basepath, size, size),
            texturemaps: TextureMaps {
                background: format!("{}/test.bkgrnd.{}x{}.png", basepath, size, size),
                overlay: format!("{}/test.overlay.{}x{}.png", basepath, size, size),
                blendcontrol: format!("{}/test.blendcontrol.{}x{}.png", basepath, size, size),
            },
            tintmap: format!("{}/test.tint.{}x{}.png", basepath, size, size),
            materialset: MaterialSetConfig::kaer_morhen(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MaterialSetConfig {
    // ------------------------------------------------------------------------
    fn prolog_village() -> Self {
        let max_textures = 0;
        let base = "_test-data_/w3.textures/levels/prolog_village/prolog_village";
        let diffuse = (0..=max_textures.min(30))
            .map(|i| format!("{}.texarray.texture_{}.png", base, i))
            .collect::<Vec<_>>();

        let normal = (0..=max_textures.min(30))
            .map(|i| format!("{}_normals.texarray.texture_{}.png", base, i))
            .collect::<Vec<_>>();

        Self {
            diffuse,
            normal,
            parameter: prolog_material_params().to_vec(),
        }
    }
    // ------------------------------------------------------------------------
    fn kaer_morhen() -> Self {
        let max_textures = 19;
        let basepath = "_test-data_/w3.textures/levels/kaer_morhen/kaer_morhen_valley";
        let diffuse = (0..=max_textures.min(19))
            .map(|i| format!("{}.texarray.texture_{}.png", basepath, i))
            .collect::<Vec<_>>();

        let normal = (0..=max_textures.min(19))
            .map(|i| format!("{}_normals.texarray.texture_{}.png", basepath, i))
            .collect::<Vec<_>>();

        Self {
            diffuse,
            normal,
            parameter: prolog_material_params()
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[allow(clippy::excessive_precision)]
fn prolog_material_params() -> [TerrainMaterialParam; 31] {
    [
        // 1
        TerrainMaterialParam {
            blend_sharpness: 0.3650000095,
            specularity_scale: 0.3225809932,
            _specularity_scale_copy: 0.3225809932,
            specularity: 0.224999994,
            specularity_base: 0.5161290169,
            ..Default::default()
        },
        // 2
        TerrainMaterialParam {
            blend_sharpness: 0.163635999,
            specularity: 0.7170000076,
            specularity_base: 0.5279999971,
            ..Default::default()
        },
        // 3
        TerrainMaterialParam {
            blend_sharpness: 0.2060610056,
            slope_normal_dampening: 0.0121210003,
            specularity: 0.2460000068,
            specularity_base: 0.5360000134,
            ..Default::default()
        },
        // 4
        TerrainMaterialParam {
            blend_sharpness: 0.3220340014,
            slope_base_dampening: 0.2711859941,
            slope_normal_dampening: 0.4848479927,
            specularity: 0.2630000114,
            specularity_base: 0.5429999828,
            ..Default::default()
        },
        // 5
        TerrainMaterialParam {
            blend_sharpness: 0.1557790041,
            specularity: 0.0542169996,
            specularity_base: 0.566264987,
            ..Default::default()
        },
        // --- 6
        TerrainMaterialParam {
            blend_sharpness: 0.1700000018,
            specularity_scale: 0.0160000008,
            specularity: 0.0903609991,
            specularity_base: 0.566264987,
            _specularity_scale_copy: 0.0160000008,
            ..Default::default()
        },
        // 7
        TerrainMaterialParam {
            specularity: 0.4169999957,
            specularity_base: 0.5099999905,
            _specularity_scale_copy: 0.3225809932,
            ..Default::default()
        },
        // 8
        TerrainMaterialParam {
            blend_sharpness: 0.5921049714,
            slope_base_dampening: 0.5789470077,
            specularity_scale: 0.0160000008,
            specularity: 0.52700001,
            specularity_base: 0.5120000243,
            _specularity_scale_copy: 0.0160000008,
            ..Default::default()
        },
        // 9
        TerrainMaterialParam {
            specularity: 0.3870970011,
            specularity_base: 0.5322579741,
            ..Default::default()
        },
        // 10
        TerrainMaterialParam {
            blend_sharpness: 0.1368419975,
            specularity_scale: 0.1700000018,
            specularity: 0.224999994,
            specularity_base: 0.4779999852,
            _specularity_scale_copy: 0.1700000018,
            ..Default::default()
        },
        // --- 11
        TerrainMaterialParam {
            blend_sharpness: 0.1684210002,
            slope_base_dampening: 0.1894740015,
            specularity_scale: 0.903226018,
            specularity: 0.3870970011,
            specularity_base: 0.5645160079,
            _specularity_scale_copy: 0.903226018,
            ..Default::default()
        },
        // 12
        TerrainMaterialParam {
            blend_sharpness: 0.5368419886,
            slope_base_dampening: 0.400000006,
            specularity_scale: 0.8548390269,
            specularity: 0.370968014,
            specularity_base: 0.596773982,
            _specularity_scale_copy: 0.8548390269,
            ..Default::default()
        },
        // 13
        TerrainMaterialParam {
            blend_sharpness: 0.1789470017,
            specularity: 0.4609999955,
            specularity_base: 0.5210000277,
            ..Default::default()
        },
        // 14
        TerrainMaterialParam {
            blend_sharpness: 0.3644070029,
            slope_base_dampening: 0.3644070029,
            specularity: 0.4838710129,
            specularity_base: 0.548386991,
            ..Default::default()
        },
        // 15
        TerrainMaterialParam {
            blend_sharpness: 0.2150000036,
            specularity_scale: 0.0869999975,
            specularity: 0.351000011,
            specularity_base: 0.4889999926,
            _specularity_scale_copy: 0.0869999975,
            ..Default::default()
        },
        // --- 16
        TerrainMaterialParam {
            slope_normal_dampening: 0.2181819975,
            specularity_scale: 0.1640000045,
            specularity: 0.4230000079,
            specularity_base: 0.3619999886,
            _specularity_scale_copy: 0.1640000045,
            ..Default::default()
        },
        // 17
        TerrainMaterialParam {
            blend_sharpness: 0.1199999973,
            specularity: 0.3790000081,
            specularity_base: 0.5490000248,
            ..Default::default()
        },
        // 18
        TerrainMaterialParam {
            blend_sharpness: 0.1757580042,
            slope_base_dampening: 0.9878789783,
            specularity: 0.5161290169,
            specularity_base: 0.5645160079,
            ..Default::default()
        },
        // 19
        TerrainMaterialParam {
            blend_sharpness: 0.1299999952,
            specularity_scale: 0.0049999999,
            specularity: 0.4720000029,
            specularity_base: 0.5870000124,
            _specularity_scale_copy: 0.0049999999,
            ..Default::default()
        },
        // 20
        TerrainMaterialParam {
            blend_sharpness: 0.1052630022,
            slope_base_dampening: 0.1016950011,
            specularity: 0.1199999973,
            specularity_base: 0.5870000124,
            ..Default::default()
        },
        // --- 21
        TerrainMaterialParam {
            specularity_scale: 0.7741940022,
            specularity: 0.1612900048,
            specularity_base: 0.419355005,
            _specularity_scale_copy: 0.7741940022,
            ..Default::default()
        },
        // 22
        TerrainMaterialParam {
            specularity: 0.3449999988,
            specularity_base: 0.5640000105,
            ..Default::default()
        },
        // 23
        TerrainMaterialParam {
            specularity: 0.4169999957,
            specularity_base: 0.5490000248,
            ..Default::default()
        },
        // 24
        TerrainMaterialParam {
            ..Default::default()
        },
        // 25
        TerrainMaterialParam {
            blend_sharpness: 0.4322029948,
            slope_base_dampening: 0.4067800045,
            specularity: 0.3619999886,
            specularity_base: 0.5149999857,
            ..Default::default()
        },
        // --- 26
        TerrainMaterialParam {
            blend_sharpness: 0.174999997,
            specularity: 0.370968014,
            specularity_base: 0.5645160079,
            falloff: 0.0549999997,
            ..Default::default()
        },
        // 27
        TerrainMaterialParam {
            blend_sharpness: 0.3449999988,
            specularity_scale: 0.2586210072,
            specularity: 0.419355005,
            specularity_base: 0.4838710129,
            falloff: 0.3620690107,
            _specularity_scale_copy: 0.2586210072,
            ..Default::default()
        },
        // 28
        TerrainMaterialParam {
            blend_sharpness: 0.1319440007,
            specularity: 0.3680000007,
            specularity_base: 0.5049999952,
            ..Default::default()
        },
        // 29
        TerrainMaterialParam {
            specularity_base: 0.5049999952,
            ..Default::default()
        },
        // 30
        TerrainMaterialParam {
            specularity_scale: 0.4720000029,
            specularity: 0.4449999928,
            specularity_base: 0.4779999852,
            _specularity_scale_copy: 0.4720000029,
            ..Default::default()
        },
        // --- 31
        TerrainMaterialParam {
            specularity: 0.4889999926,
            specularity_base: 0.5490000248,
            ..Default::default()
        },
    ]
}
// ----------------------------------------------------------------------------
