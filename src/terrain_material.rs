// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct TerrainMaterialParam {
    pub blend_sharpness: f32,
    pub slope_base_dampening: f32,
    pub slope_normal_dampening: f32,
    pub specularity_scale: f32,
    pub specularity: f32,
    pub specularity_base: f32,
    pub _specularity_scale_copy: f32,
    pub falloff: f32,
}
// ----------------------------------------------------------------------------
// material params
// ----------------------------------------------------------------------------
impl Default for TerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // TODO check defaults
        Self {
            blend_sharpness: 0.0,
            slope_base_dampening: 0.0,
            slope_normal_dampening: 0.5,
            specularity_scale: 0.0,
            specularity: 0.0,
            specularity_base: 0.0,
            _specularity_scale_copy: 0.0,
            falloff: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
