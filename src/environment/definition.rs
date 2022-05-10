// ----------------------------------------------------------------------------
use super::ColorCurveEntry;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct EnvironmentConfig {
    pub sun: SunConfig,
}
// ----------------------------------------------------------------------------
pub(super) struct SunConfig {
    pub color: Vec<ColorCurveEntry>,
}
// ----------------------------------------------------------------------------
impl EnvironmentConfig {
    // ------------------------------------------------------------------------
    pub fn load(path: &str) -> Result<Self, String> {
        match path {
            "environment/definitions/env_prologue/env_prolog_colors_v1_b_sunset.env" => {
                prolog_env_settings()
            }
            "environment/definitions/kaer_morhen/kaer_morhen_global/env_kaer_morhen_v09_tm.env" => {
                kaer_morhen_env_settings()
            }
            _ => Err(format!(
                "error loading enviroment settings: path [{}] not found.",
                path
            )),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// defaults
// ----------------------------------------------------------------------------
impl Default for SunConfig {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        Self {
            color: vec![ColorCurveEntry::try_from(("00:00", 255.0, 255.0, 255.0, 100.0)).unwrap()],
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// hardcoded test values
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[allow(clippy::excessive_precision, clippy::approx_constant)]
fn prolog_env_settings() -> Result<EnvironmentConfig, String> {
    Ok(EnvironmentConfig {
        sun: SunConfig {
            color: vec![
                ColorCurveEntry::try_from(("00:28", 61.0, 113.0, 154.0, 85.08))?,
                ColorCurveEntry::try_from(("02:26", 108.0, 151.0, 180.0, 85.08))?,
                ColorCurveEntry::try_from(("03:07", 73.0, 103.0, 124.0, 85.08))?,
                ColorCurveEntry::try_from(("03:19", 8.0, 9.0, 10.0, 85.08))?,
                ColorCurveEntry::try_from(("03:26", 15.0, 11.0, 4.0, 85.08))?,
                ColorCurveEntry::try_from(("03:38", 252.002, 196.002, 123.002, 85.08))?,
                ColorCurveEntry::try_from(("03:47", 252.0, 196.0, 123.0, 85.08))?,
                ColorCurveEntry::try_from(("04:12", 252.0, 196.0, 123.0, 85.08))?,
                ColorCurveEntry::try_from(("09:57", 196.0, 175.0, 152.0, 85.08))?,
                ColorCurveEntry::try_from(("14:25", 194.0, 173.0, 150.0, 85.08))?,
                ColorCurveEntry::try_from(("16:10", 194.053, 173.42, 150.482, 85.08))?,
                ColorCurveEntry::try_from(("18:35", 187.0, 141.0, 53.0, 85.08))?,
                ColorCurveEntry::try_from(("19:17", 202.0, 136.0, 9.0, 85.08))?,
                ColorCurveEntry::try_from(("20:04", 255.0, 145.0, 11.0, 85.08))?,
                ColorCurveEntry::try_from(("20:55", 51.0, 26.0, 17.0, 85.08))?,
                ColorCurveEntry::try_from(("21:16", 31.363, 28.431, 26.726, 85.08))?,
                ColorCurveEntry::try_from(("21:20", 7.0, 7.0, 7.0, 85.08))?,
                ColorCurveEntry::try_from(("21:33", 7.0, 7.0, 7.0, 85.08))?,
                ColorCurveEntry::try_from(("21:41", 42.732, 63.045, 78.565, 85.08))?,
                ColorCurveEntry::try_from(("22:02", 57.0, 105.0, 143.0, 85.08))?,
                ColorCurveEntry::try_from(("22:19", 57.0, 105.0, 143.0, 85.08))?,
                ColorCurveEntry::try_from(("22:45", 57.0, 105.0, 143.0, 85.08))?,
                ColorCurveEntry::try_from(("23:06", 61.0, 113.0, 155.0, 85.08))?,
            ]
        }
    })
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
fn kaer_morhen_env_settings() -> Result<EnvironmentConfig, String> {
    Ok(EnvironmentConfig {
        sun: SunConfig {
            color: vec![
                ColorCurveEntry::try_from(("01:53", 45.897, 67.873, 81.837, 70.026))?,
                ColorCurveEntry::try_from(("02:53", 22.0, 38.0, 44.0, 70.026))?,
                ColorCurveEntry::try_from(("03:54", 254.0, 195.0, 124.0, 70.025))?,
                ColorCurveEntry::try_from(("09:43", 245.0, 230.0, 205.0, 70.026))?,
                ColorCurveEntry::try_from(("11:40", 245.0, 230.0, 205.0, 70.026))?,
                ColorCurveEntry::try_from(("14:34", 245.0, 230.0, 205.0, 70.026))?,
                ColorCurveEntry::try_from(("17:57", 254.0, 158.0, 124.0, 70.026))?,
                ColorCurveEntry::try_from(("19:52", 94.788, 111.535, 127.092, 70.026))?,
                ColorCurveEntry::try_from(("21:53", 45.897, 67.871, 81.837, 70.026))?,
            ],
        }
    })
}
// ----------------------------------------------------------------------------
