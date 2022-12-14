// ----------------------------------------------------------------------------
// 256 seems to be a good compromise for 16k x 16k terrains.
pub const TILE_SIZE: u32 = 256;
// Since a clipmap level is assigned to a tile which is completely covered by
// clipmap level the clipmap size should be at least 4 * TILE_SIZE to make sure
// that at least 3 tiles (>= 1.5 tiles in all directions from camera) are
// highest res. Good value is 1024.
// Note: atm MAX is 1024 since this amount of rays are used to compute shadows
pub const CLIPMAP_SIZE: u32 = TILE_SIZE * 4;
// Granularity of clipmap view positions in full data. Since clipmap levels are
// assigned to tiles this should be the same size (lower granularity will update
// clipmap data more often but tiles will only be updated if they are fully
// covered).
pub const CLIPMAP_GRANULARITY: u32 = TILE_SIZE;
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
    /// clipmnap levels
    clipmap_levels: u8,
    /// currently assigned materialset info
    materialset: MaterialSetConfig,
    /// assigned environment definition
    environment: Option<String>,
}
// ----------------------------------------------------------------------------
use bevy::math::{uvec2, vec2, UVec2, Vec2};

use crate::terrain_material::{MaterialSlot, TerrainMaterialParam};
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
    pub fn clipmap_levels(&self) -> u8 {
        self.clipmap_levels
    }
    // ------------------------------------------------------------------------
    pub fn max_clipmap_level(&self) -> u8 {
        self.clipmap_levels - 1
    }
    // ------------------------------------------------------------------------
    pub fn materialset(&self) -> &MaterialSetConfig {
        &self.materialset
    }
    // ------------------------------------------------------------------------
    pub fn environment_definition(&self) -> Option<&str> {
        self.environment.as_deref()
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn world_pos_to_map_pos(&self, pos: Vec2) -> UVec2 {
        let map_offset = self.map_offset();

        ((pos - map_offset) / self.resolution)
            .round()
            .as_uvec2()
            // clamp to data size
            .min(uvec2(self.map_size - 1, self.map_size - 1))
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
    pub fn textures(&self) -> impl Iterator<Item = (MaterialSlot, &str, &str)> {
        self.diffuse
            .iter()
            .zip(self.normal.iter())
            .enumerate()
            .map(|(i, (diffuse, normal))| {
                (
                    MaterialSlot::from(i as u8),
                    diffuse.as_str(),
                    normal.as_str(),
                )
            })
    }
    // ------------------------------------------------------------------------
    pub fn parameters(&self) -> impl Iterator<Item = (MaterialSlot, &TerrainMaterialParam)> {
        self.parameter
            .iter()
            .enumerate()
            .map(|(i, p)| (MaterialSlot::from(i as u8), p))
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
            clipmap_levels: 3,
            materialset: MaterialSetConfig::prolog_village(),
            environment: Some("environment/definitions/env_prologue/env_prolog_colors_v1_b_sunset.env".to_string()),
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
            clipmap_levels: 5,
            materialset: MaterialSetConfig::kaer_morhen(),
            environment: Some("environment/definitions/kaer_morhen/kaer_morhen_global/env_kaer_morhen_v09_tm.env".to_string()),
        }
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn bevy_example() -> Self {
        let basepath = "_test-data_/terrain/";
        let _hubid = "bevy";
        let size = 4096;
        Self {
            name: format!("Bevy ({} x {})", size, size),
            terrain_size: size as f32 / 2.0,
            map_size: size,
            resolution: 0.5,
            min_height: -37.0,
            max_height: 245.0,
            heightmap: format!("{}/bevy.heightmap.{}x{}.png", basepath, size, size),
            texturemaps: TextureMaps {
                background: format!("{}/bevy.bkgrnd.{}x{}.png", basepath, size, size),
                overlay: format!("{}/bevy.overlay.{}x{}.png", basepath, size, size),
                blendcontrol: format!("{}/bevy.blendcontrol.{}x{}.png", basepath, size, size),
            },
            tintmap: format!("{}/bevy.tint.{}x{}.png", basepath, size, size),
            clipmap_levels: 3,
            materialset: MaterialSetConfig::bevy_example(),
            environment: Some("environment/definitions/env_prologue/env_prolog_colors_v1_b_sunset.env".to_string()),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MaterialSetConfig {
    // ------------------------------------------------------------------------
    fn prolog_village() -> Self {
        let max_textures = 30;
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
            parameter: kaer_morhen_material_params()
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>(),
        }
    }
    // ------------------------------------------------------------------------
    fn bevy_example() -> Self {
        let max_textures = 16;
        let basepath = "_test-data_/textures/";
        let diffuse = (0..=max_textures.min(16))
            .map(|i| format!("{}texture.diffuse_{}.png", basepath, i))
            .collect::<Vec<_>>();

        let normal = (0..=max_textures.min(16))
            .map(|i| format!("{}texture.normal_{}.png", basepath, i))
            .collect::<Vec<_>>();

        Self {
            diffuse,
            normal,
            parameter: bevy_example_material_params()
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
#[allow(clippy::excessive_precision)]
fn kaer_morhen_material_params() -> [TerrainMaterialParam; 30] {
    [
        // 01:
        TerrainMaterialParam {
            blend_sharpness: 0.3797470033,
            slope_base_dampening: 0.6204379797,
            specularity_scale: 0.419355005,
            _specularity_scale_copy: 0.419355005,
            specularity: 0.4032259881,
            specularity_base: 0.451613009,
            ..Default::default()
        },
        // 02:
        TerrainMaterialParam {
            blend_sharpness: 0.075000003,
            slope_base_dampening: 1.0,
            specularity: 0.6774190068,
            specularity_base: 0.548386991,
            ..Default::default()
        },
        // 03:
        TerrainMaterialParam {
            blend_sharpness: 0.1389999986,
            slope_base_dampening: 0.2599999905,
            specularity: 0.577113986,
            specularity_base: 0.5174130201,
            ..Default::default()
        },
        // 04:
        TerrainMaterialParam {
            specularity: 0.4776119888,
            specularity_base: 0.5519999862,
            ..Default::default()
        },
        // 05:
        TerrainMaterialParam {
            blend_sharpness: 0.139240995,
            slope_base_dampening: 1.0,
            specularity: 0.6290320158,
            specularity_base: 0.5479999781,
            ..Default::default()
        },
        // 06:
        TerrainMaterialParam {
            slope_base_dampening: 1.0,
            specularity: 0.5671640038,
            specularity_base: 0.562188983,
            ..Default::default()
        },
        // 07:
        TerrainMaterialParam {
            blend_sharpness: 0.2300000042,
            slope_base_dampening: 1.0,
            specularity: 0.6069650054,
            specularity_base: 0.5619999766,
            ..Default::default()
        },
        // 08:
        TerrainMaterialParam {
            blend_sharpness: 0.4487800002,
            slope_base_dampening: 0.2150000036,
            specularity: 0.75,
            specularity_base: 0.5174130201,
            ..Default::default()
        },
        // 09:
        TerrainMaterialParam {
            blend_sharpness: 0.2303919941,
            slope_base_dampening: 0.112999998,
            specularity: 0.4925369918,
            specularity_base: 0.52700001,
            ..Default::default()
        },
        // 10:
        TerrainMaterialParam {
            blend_sharpness: 0.2025319934,
            slope_base_dampening: 0.025316,
            slope_normal_dampening: 0.3759999871,
            specularity_scale: 0.3870970011,
            _specularity_scale_copy: 0.3870970011,
            specularity: 0.1935479939,
            specularity_base: 0.467741996,
            ..Default::default()
        },
        // 11:
        TerrainMaterialParam {
            blend_sharpness: 0.177214995,
            slope_base_dampening: 0.4550000131,
            specularity_scale: 0.5024880171,
            _specularity_scale_copy: 0.5024880171,
            specularity: 0.3980099857,
            specularity_base: 0.4420000017,
            ..Default::default()
        },
        // 12:
        TerrainMaterialParam {
            blend_sharpness: 0.1150000021,
            slope_base_dampening: 0.5690000057,
            specularity: 0.4726369977,
            specularity_base: 0.5469999909,
            ..Default::default()
        },
        // 13:
        TerrainMaterialParam {
            blend_sharpness: 0.150820002,
            slope_base_dampening: 0.1770000011,
            slope_normal_dampening: 0.376812011,
            specularity: 0.4676620066,
            specularity_base: 0.5009999871,
            ..Default::default()
        },
        // 14:
        TerrainMaterialParam {
            blend_sharpness: 0.0759489983,
            slope_base_dampening: 0.1449999958,
            specularity: 0.5370000005,
            specularity_base: 0.5,
            ..Default::default()
        },
        // 15:
        TerrainMaterialParam {
            blend_sharpness: 0.3670000136,
            slope_base_dampening: 0.3409999907,
            specularity_scale: 0.1442790031,
            _specularity_scale_copy: 0.1442790031,
            specularity: 0.5273630023,
            specularity_base: 0.5070000291,
            ..Default::default()
        },
        // 16:
        TerrainMaterialParam {
            blend_sharpness: 0.0759489983,
            slope_base_dampening: 0.1889999956,
            specularity: 0.4079599977,
            specularity_base: 0.5170000196,
            ..Default::default()
        },
        // 17:
        TerrainMaterialParam {
            blend_sharpness: 0.151898995,
            slope_base_dampening: 0.2399999946,
            specularity_scale: 0.1612900048,
            _specularity_scale_copy: 0.1612900048,
            specularity: 0.4726369977,
            specularity_base: 0.5161290169,
            ..Default::default()
        },
        // 18:
        TerrainMaterialParam {
            blend_sharpness: 0.0886079967,
            slope_base_dampening: 0.3030000031,
            specularity: 0.548386991,
            specularity_base: 0.5161290169,
            ..Default::default()
        },
        // 19:
        TerrainMaterialParam {
            blend_sharpness: 0.0632909983,
            slope_base_dampening: 1.0,
            specularity: 0.4354839921,
            specularity_base: 0.5070000291,
            ..Default::default()
        },
        // 20:
        TerrainMaterialParam {
            blend_sharpness: 0.3670890033,
            slope_base_dampening: 1.0,
            specularity_scale: 0.7580649853,
            _specularity_scale_copy: 0.7580649853,
            specularity: 0.2096769959,
            specularity_base: 0.419355005,
            ..Default::default()
        },
        // 21:
        TerrainMaterialParam {
            blend_sharpness: 0.0886079967,
            slope_base_dampening: 0.189872995,
            specularity_scale: 0.3283579946,
            _specularity_scale_copy: 0.3283579946,
            specularity: 0.5323380232,
            specularity_base: 0.4970000088,
            ..Default::default()
        },
        // 22:
        TerrainMaterialParam {
            blend_sharpness: 0.1139239967,
            slope_base_dampening: 0.2405059934,
            specularity: 0.288556993,
            specularity_base: 0.5960000157,
            ..Default::default()
        },
        // 23:
        TerrainMaterialParam {
            blend_sharpness: 0.0506329983,
            slope_base_dampening: 0.3037970066,
            specularity: 0.3930349946,
            specularity_base: 0.4925369918,
            ..Default::default()
        },
        // 24:
        TerrainMaterialParam {
            blend_sharpness: 0.1012659967,
            slope_base_dampening: 0.9240509868,
            specularity: 0.6119400263,
            specularity_base: 0.5170000196,
            ..Default::default()
        },
        // 25:
        TerrainMaterialParam {
            blend_sharpness: 0.1012659967,
            slope_base_dampening: 0.177214995,
            specularity: 0.5671640038,
            specularity_base: 1.0,
            ..Default::default()
        },
        // 26:
        TerrainMaterialParam {
            blend_sharpness: 0.0886079967,
            slope_base_dampening: 1.0,
            specularity: 0.3980099857,
            specularity_base: 0.4925369918,
            ..Default::default()
        },
        // 27:
        TerrainMaterialParam {
            blend_sharpness: 0.0886079967,
            slope_base_dampening: 0.4854010046,
            specularity: 0.8358209729,
            specularity_base: 0.5024880171,
            ..Default::default()
        },
        // 28:
        TerrainMaterialParam {
            blend_sharpness: 0.1265819967,
            slope_base_dampening: 1.0,
            specularity: 0.4776119888,
            specularity_base: 0.5124379992,
            ..Default::default()
        },
        // 29:
        TerrainMaterialParam {
            blend_sharpness: 0.189872995,
            slope_base_dampening: 0.987342,
            specularity: 0.5373129845,
            specularity_base: 0.5223879814,
            ..Default::default()
        },
        // 30:
        TerrainMaterialParam {
            specularity_scale: 0.5970150232,
            _specularity_scale_copy: 0.5970150232,
            specularity: 0.3134329915,
            specularity_base: 0.4726369977,
            ..Default::default()
        },
    ]
}
// ----------------------------------------------------------------------------
#[allow(clippy::excessive_precision)]
fn bevy_example_material_params() -> [TerrainMaterialParam; 16] {
    [
        // 1 grass ~ mossy
        TerrainMaterialParam {
            blend_sharpness: 0.45,
            slope_base_dampening: 0.22,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 2 rocks
        TerrainMaterialParam {
            blend_sharpness: 0.35,
            slope_base_dampening: 0.35,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 3 small rocks on rocks
        TerrainMaterialParam {
            blend_sharpness: 0.08,
            slope_base_dampening: 0.3,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 4 sand
        TerrainMaterialParam {
            blend_sharpness: 0.12,
            slope_base_dampening: 0.5,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 5 sand curly
        TerrainMaterialParam {
            blend_sharpness: 0.5,
            slope_base_dampening: 0.5,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 6 stones bkgrnd
        TerrainMaterialParam {
            blend_sharpness: 0.2,
            slope_base_dampening: 0.4,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 7 grass ~ hard cover rocks, as bkgrnd will be hidden on horizontal terrain
        TerrainMaterialParam {
            blend_sharpness: 0.3,
            slope_base_dampening: 1.0,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 8 rock dark
        TerrainMaterialParam {
            blend_sharpness: 0.5,
            slope_base_dampening: 1.0,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 9 rock grey
        TerrainMaterialParam {
            blend_sharpness: 0.5,
            slope_base_dampening: 1.0,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 10 rock light grey
        TerrainMaterialParam {
            blend_sharpness: 0.5,
            slope_base_dampening: 1.0,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 11 smaller rocks dark
        TerrainMaterialParam {
            blend_sharpness: 0.7,
            slope_base_dampening: 0.7,
            slope_normal_dampening: 1.0,
            ..Default::default()
        },
        // 12 smaller rocks grey
        TerrainMaterialParam {
            blend_sharpness: 0.7,
            slope_base_dampening: 0.7,
            slope_normal_dampening: 1.0,
            ..Default::default()
        },
        // 13 smaller rocks light greay
        TerrainMaterialParam {
            blend_sharpness: 0.7,
            slope_base_dampening: 0.7,
            slope_normal_dampening: 1.0,
            ..Default::default()
        },
        // 14 pebbles dark
        TerrainMaterialParam {
            blend_sharpness: 0.3,
            slope_base_dampening: 0.2,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 15 pebbles grey
        TerrainMaterialParam {
            blend_sharpness: 0.3,
            slope_base_dampening: 0.2,
            slope_normal_dampening: 0.5,
            ..Default::default()
        },
        // 16 pebbles light grey
        TerrainMaterialParam {
            blend_sharpness: 0.3,
            slope_base_dampening: 0.2,
            slope_normal_dampening: 0.5,
            ..Default::default()
        }
    ]
}
// ----------------------------------------------------------------------------
