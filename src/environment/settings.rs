// ----------------------------------------------------------------------------
use super::interpolation::ColorInterpolation;

use super::EnvironmentConfig;
// ----------------------------------------------------------------------------
/// environment settings prepared for interpolated sampling
pub struct EnvironmentSettings {
    pub sun: SunSettings,
}
// ----------------------------------------------------------------------------
pub struct SunSettings {
    pub color: ColorInterpolation,
}
// ----------------------------------------------------------------------------
// config -> settings
// ----------------------------------------------------------------------------
impl From<EnvironmentConfig> for EnvironmentSettings {
    // ------------------------------------------------------------------------
    fn from(conf: EnvironmentConfig) -> Self {
        Self {
            sun: SunSettings {
                color: ColorInterpolation::from(conf.sun.color),
            },
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
