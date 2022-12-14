//
// progress tracking of (async) cmds
//
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Eq, Debug)]
/// IMPORTANT: implemented eq and hash consider only the enum variant and ignore
/// payload!
pub enum TrackedProgress {
    LoadHeightmap(bool),
    LoadTextureMap(bool),
    LoadTintMap(bool),
    GenerateClipmap(bool),
    GeneratedHeightmapNormals(usize, usize),
    GeneratedTerrainErrorMaps(usize, usize),
    MergedTerrainErrorMapSeams(usize, usize),
    GenerateTerrainTiles(bool),
    GeneratedTerrainMeshes(usize, usize),
    LoadTerrainMaterialSet(usize, usize),
    Ignored,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TrackedTaskname(String);
// ----------------------------------------------------------------------------
impl TrackedProgress {
    // ------------------------------------------------------------------------
    pub fn is_finished(&self) -> bool {
        match self {
            Self::LoadHeightmap(b)
            | Self::LoadTextureMap(b)
            | Self::LoadTintMap(b)
            | Self::GenerateClipmap(b)
            | Self::GenerateTerrainTiles(b) => *b,
            Self::GeneratedHeightmapNormals(a, b)
            | Self::GeneratedTerrainErrorMaps(a, b)
            | Self::MergedTerrainErrorMapSeams(a, b)
            | Self::GeneratedTerrainMeshes(a, b)
            | Self::LoadTerrainMaterialSet(a, b) => *a == *b,
            Self::Ignored => true,
        }
    }
    // ------------------------------------------------------------------------
    pub fn progress(&self) -> f32 {
        match self {
            Self::LoadHeightmap(b)
            | Self::LoadTextureMap(b)
            | Self::LoadTintMap(b)
            | Self::GenerateClipmap(b)
            | Self::GenerateTerrainTiles(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::GeneratedHeightmapNormals(a, b)
            | Self::GeneratedTerrainErrorMaps(a, b)
            | Self::MergedTerrainErrorMapSeams(a, b)
            | Self::GeneratedTerrainMeshes(a, b)
            | Self::LoadTerrainMaterialSet(a, b) => *a as f32 / *b as f32,
            Self::Ignored => 1.0,
        }
    }
    // ------------------------------------------------------------------------
    fn format_progress(msg: &str, percentage: f32) -> String {
        format!("{}...{}%", msg, (percentage * 100.0).trunc())
    }
    // ------------------------------------------------------------------------
    pub fn progress_msg(&self) -> String {
        match self {
            Self::LoadHeightmap(_) => "loading heighmap...".to_string(),
            Self::LoadTextureMap(_) => "loading texturing map...".to_string(),
            Self::LoadTintMap(_) => "loading tintmap...".to_string(),
            Self::GenerateClipmap(_) => "generating clipmap...".to_string(),
            Self::GeneratedHeightmapNormals(_, _) => {
                Self::format_progress("generating normals", self.progress())
            }
            Self::GenerateTerrainTiles(_) => "generating tiles...".to_string(),
            Self::GeneratedTerrainErrorMaps(_, _) => {
                Self::format_progress("generating error maps", self.progress())
            }
            Self::MergedTerrainErrorMapSeams(_, _) => {
                Self::format_progress("merging error map seams", self.progress())
            }
            Self::GeneratedTerrainMeshes(_, _) => {
                Self::format_progress("generating tile meshes", self.progress())
            }
            Self::LoadTerrainMaterialSet(a, b) => {
                format!("loading materials...{}/{}", a, b)
            }
            Self::Ignored => String::default(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn finished_msg(&self) -> &str {
        match self {
            Self::LoadHeightmap(_) => "heightmap loaded.",
            Self::LoadTextureMap(_) => "texturing map loaded.",
            Self::LoadTintMap(_) => "tintmap loaded.",
            Self::GenerateClipmap(_) => "clipmap generated.",
            Self::GeneratedHeightmapNormals(_, _) => "heightmap normals generated.",
            Self::GenerateTerrainTiles(_) => "mesh tile info generated.",
            Self::GeneratedTerrainErrorMaps(_, _) => "terrain error maps generation finished.",
            Self::MergedTerrainErrorMapSeams(_, _) => "terrain error map seams merged.",
            Self::GeneratedTerrainMeshes(_, _) => "terrain mesh generation finished.",
            Self::LoadTerrainMaterialSet(_, _) => "materials loaded.",
            Self::Ignored => "",
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::cmp;
use std::hash;

impl hash::Hash for TrackedProgress {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        use TrackedProgress::*;
        match self {
            Ignored => state.write_u8(0),
            LoadHeightmap(_) => state.write_u8(1),
            LoadTextureMap(_) => state.write_u8(2),
            LoadTintMap(_) => state.write_u8(3),
            GenerateClipmap(_) => state.write_u8(4),
            GeneratedHeightmapNormals(_, _) => state.write_u8(5),
            GenerateTerrainTiles(_) => state.write_u8(6),
            GeneratedTerrainErrorMaps(_, _) => state.write_u8(7),
            MergedTerrainErrorMapSeams(_, _) => state.write_u8(8),
            GeneratedTerrainMeshes(_, _) => state.write_u8(9),
            LoadTerrainMaterialSet(_, _) => state.write_u8(10),
        }
    }
}

impl cmp::PartialEq for TrackedProgress {
    fn eq(&self, other: &Self) -> bool {
        use TrackedProgress::*;
        match self {
            Ignored => matches!(other, Ignored),
            LoadHeightmap(_) => matches!(other, LoadHeightmap(_)),
            LoadTextureMap(_) => matches!(other, LoadTextureMap(_)),
            LoadTintMap(_) => matches!(other, LoadTintMap(_)),
            GenerateClipmap(_) => matches!(other, GenerateClipmap(_)),
            GeneratedHeightmapNormals(_, _) => matches!(other, GeneratedHeightmapNormals(_, _)),
            GenerateTerrainTiles(_) => matches!(other, GenerateTerrainTiles(_)),
            GeneratedTerrainErrorMaps(_, _) => matches!(other, GeneratedTerrainErrorMaps(_, _)),
            MergedTerrainErrorMapSeams(_, _) => matches!(other, MergedTerrainErrorMapSeams(_, _)),
            GeneratedTerrainMeshes(_, _) => matches!(other, GeneratedTerrainMeshes(_, _)),
            LoadTerrainMaterialSet(_, _) => matches!(other, LoadTerrainMaterialSet(_, _)),
        }
    }
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
impl TrackedTaskname {
    // ------------------------------------------------------------------------
    pub fn as_str(&self) -> Option<&str> {
        if self.0.is_empty() {
            None
        } else {
            Some(&self.0)
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl<'s> From<&'s str> for TrackedTaskname {
    fn from(s: &'s str) -> Self {
        TrackedTaskname(s.to_string())
    }
}
// ----------------------------------------------------------------------------
