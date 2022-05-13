// ----------------------------------------------------------------------------
use super::{ColorCurveEntry, ScalarCurveEntry};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct EnvironmentConfig {
    pub sun: SunConfig,
    pub fog: FogConfig,
}
// ----------------------------------------------------------------------------
pub(super) struct SunConfig {
    pub color: Vec<ColorCurveEntry>,
}
// ----------------------------------------------------------------------------
pub(super) struct FogConfig {
    pub appear_distance: Vec<ScalarCurveEntry>,
    pub appear_range: Vec<ScalarCurveEntry>,
    pub color_front: Vec<ColorCurveEntry>,
    pub color_middle: Vec<ColorCurveEntry>,
    pub color_back: Vec<ColorCurveEntry>,
    pub density: Vec<ScalarCurveEntry>,
    pub final_exp: Vec<ScalarCurveEntry>,
    pub distance_clamp: Vec<ScalarCurveEntry>,
    pub vertical_offset: Vec<ScalarCurveEntry>,
    pub vertical_density: Vec<ScalarCurveEntry>,
    pub vertical_density_light_front: Vec<ScalarCurveEntry>,
    pub vertical_density_light_back: Vec<ScalarCurveEntry>,
    // sky_denity_scale: Vec<ScalarCurveEntry>,
    // clouds_density_scale: Vec<ScalarCurveEntry>,
    // sky_vertical_density_light_front_scale: Vec<ScalarCurveEntry>,
    // sky_vertical_density_light_back_scale: Vec<ScalarCurveEntry>,
    pub vertical_density_rim_range: Vec<ScalarCurveEntry>,
    pub custom_color: Vec<ColorCurveEntry>,
    pub custom_color_start: Vec<ScalarCurveEntry>,
    pub custom_color_range: Vec<ScalarCurveEntry>,
    pub custom_amount_scale: Vec<ScalarCurveEntry>,
    pub custom_amount_scale_start: Vec<ScalarCurveEntry>,
    pub custom_amount_scale_range: Vec<ScalarCurveEntry>,
    pub aerial_color_front: Vec<ColorCurveEntry>,
    pub aerial_color_middle: Vec<ColorCurveEntry>,
    pub aerial_color_back: Vec<ColorCurveEntry>,
    pub aerial_final_exp: Vec<ScalarCurveEntry>,
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
impl Default for FogConfig {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        let color = ColorCurveEntry::try_from(("00:00", 255.0, 255.0, 255.0, 100.0)).unwrap();
        let scalar_small = ScalarCurveEntry::try_from(("00:00", 0.0)).unwrap();
        let scalar_big = ScalarCurveEntry::try_from(("00:00", 32768.0)).unwrap();
        Self {
            appear_distance: vec![scalar_big.clone()],
            appear_range: vec![scalar_small.clone()],
            color_front: vec![color.clone()],
            color_middle: vec![color.clone()],
            color_back: vec![color.clone()],
            density: vec![scalar_small.clone()],
            final_exp: vec![scalar_small.clone()],
            distance_clamp: vec![scalar_big.clone()],
            vertical_offset: vec![scalar_big.clone()],
            vertical_density: vec![scalar_small.clone()],
            vertical_density_light_front: vec![scalar_small.clone()],
            vertical_density_light_back: vec![scalar_small.clone()],
            vertical_density_rim_range: vec![scalar_small.clone()],
            custom_color: vec![color.clone()],
            custom_color_start: vec![scalar_big.clone()],
            custom_color_range: vec![scalar_big],
            custom_amount_scale: vec![scalar_small.clone()],
            custom_amount_scale_start: vec![scalar_small.clone()],
            custom_amount_scale_range: vec![scalar_small.clone()],
            aerial_color_front: vec![color.clone()],
            aerial_color_middle: vec![color.clone()],
            aerial_color_back: vec![color],
            aerial_final_exp: vec![scalar_small],
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
        },
        fog: FogConfig {
            appear_distance:    vec![ScalarCurveEntry::try_from(("20:56", 1.0))?],
            appear_range:       vec![ScalarCurveEntry::try_from(("20:56", 66.0657348633))?],
            color_front: vec![
                ColorCurveEntry::try_from(("00:22", 37.012, 64.993, 75.993, 10.378))?,
                ColorCurveEntry::try_from(("01:19", 37.0, 65.0, 76.0, 10.380))?,
                ColorCurveEntry::try_from(("03:18", 28.0, 50.0, 58.0, 10.359))?,
                ColorCurveEntry::try_from(("03:25", 28.0, 51.0, 59.0, 10.358))?,
                ColorCurveEntry::try_from(("03:26", 62.0, 36.0, 20.0, 10.359))?,
                ColorCurveEntry::try_from(("03:38", 120.0, 86.0, 35.0, 10.358))?,
                ColorCurveEntry::try_from(("03:53", 130.0, 93.0, 37.0, 10.358))?,
                ColorCurveEntry::try_from(("04:57", 145.0, 96.0, 25.0, 10.357))?,
                ColorCurveEntry::try_from(("06:33", 167.0, 158.0, 120.0, 10.441))?,
                ColorCurveEntry::try_from(("07:54", 164.0, 190.0, 204.0, 10.359))?,
                ColorCurveEntry::try_from(("13:04", 162.0, 189.0, 213.0, 10.420))?,
                ColorCurveEntry::try_from(("15:29", 162.0, 189.0, 213.0, 10.420))?,
                ColorCurveEntry::try_from(("18:59", 118.0, 74.0, 48.0, 10.452))?,
                ColorCurveEntry::try_from(("19:39", 126.0, 80.0, 52.0, 10.452))?,
                ColorCurveEntry::try_from(("20:29", 94.0, 65.0, 47.0, 10.452))?,
                ColorCurveEntry::try_from(("20:47", 79.0, 57.0, 43.0, 10.452))?,
                ColorCurveEntry::try_from(("21:29", 36.0, 53.0, 58.0, 10.42))?,
                ColorCurveEntry::try_from(("22:30", 30.0, 70.0, 81.0, 10.42))?,
            ],
            color_middle: vec![
                ColorCurveEntry::try_from(("00:17", 30.0, 53.0, 62.0, 8.999))?,
                ColorCurveEntry::try_from(("01:28", 37.0, 65.0, 76.0, 8.998))?,
                ColorCurveEntry::try_from(("03:10", 20.335, 55.741, 66.741, 7.945))?,
                ColorCurveEntry::try_from(("03:54", 79.0, 86.0, 91.0, 9.005))?,
                ColorCurveEntry::try_from(("04:13", 75.0, 88.0, 97.0, 9.009))?,
                ColorCurveEntry::try_from(("05:10", 85.0, 104.0, 118.0, 9.016))?,
                ColorCurveEntry::try_from(("06:30", 100.0, 132.0, 155.0, 9.018))?,
                ColorCurveEntry::try_from(("16:17", 135.0, 174.0, 209.0, 9.014))?,
                ColorCurveEntry::try_from(("19:31", 67.0, 86.0, 95.0, 9.021))?,
                ColorCurveEntry::try_from(("20:47", 58.0, 69.0, 74.0, 8.016))?,
                ColorCurveEntry::try_from(("21:32", 31.0, 47.0, 51.0, 12.168))?,
                ColorCurveEntry::try_from(("22:31", 30.0, 70.0, 81.0, 9.007))?,
            ],
            color_back: vec![
                ColorCurveEntry::try_from(("00:22", 30.0, 53.0, 62.0, 7.966))?,
                ColorCurveEntry::try_from(("01:29", 37.0, 65.0, 76.0, 7.965))?,
                ColorCurveEntry::try_from(("03:15", 19.0, 55.0, 66.0, 7.944))?,
                ColorCurveEntry::try_from(("03:49", 46.0, 71.0, 83.0, 8.086))?,
                ColorCurveEntry::try_from(("04:10", 46.0, 71.0, 83.0, 8.249))?,
                ColorCurveEntry::try_from(("04:56", 70.0, 89.0, 104.0, 9.016))?,
                ColorCurveEntry::try_from(("06:36", 119.0, 154.0, 184.0, 8.471))?,
                ColorCurveEntry::try_from(("16:14", 119.0, 154.0, 184.0, 8.471))?,
                ColorCurveEntry::try_from(("19:30", 55.0, 72.0, 81.0, 7.925))?,
                ColorCurveEntry::try_from(("20:43", 55.0, 72.0, 81.0, 7.925))?,
                ColorCurveEntry::try_from(("21:32", 31.0, 47.0, 51.0, 12.168))?,
                ColorCurveEntry::try_from(("22:31", 30.0, 70.0, 81.0, 7.952))?,
            ],
            density: vec![
                ScalarCurveEntry::try_from(("03:14", 0.001))?,
                ScalarCurveEntry::try_from(("06:59", 0.001))?,
                ScalarCurveEntry::try_from(("16:59", 0.001))?,
                ScalarCurveEntry::try_from(("19:19", 0.0020009577))?,
            ],
            final_exp: vec![
                ScalarCurveEntry::try_from(("02:21", 0.449162364))?,
                ScalarCurveEntry::try_from(("03:28", 0.7669311166))?,
                ScalarCurveEntry::try_from(("03:56", 0.8201212287))?,
                ScalarCurveEntry::try_from(("04:38", 0.8413972259))?,
                ScalarCurveEntry::try_from(("07:35", 1.2095916271))?,
                ScalarCurveEntry::try_from(("12:00", 0.9761633873))?,
                ScalarCurveEntry::try_from(("16:54", 0.9761630297))?,
                ScalarCurveEntry::try_from(("20:54", 0.7475311756))?,
                ScalarCurveEntry::try_from(("21:57", 0.5865037441))?,
            ],
            distance_clamp: vec![
                ScalarCurveEntry::try_from(("11:31", 16707.900390625))?,
            ],
            vertical_offset: vec![
                ScalarCurveEntry::try_from(("12:13", 39.4300003052))?,
            ],
            vertical_density: vec![
                ScalarCurveEntry::try_from(("11:59", -0.0232305992))?,
            ],
            vertical_density_light_front: vec![
                ScalarCurveEntry::try_from(("10:13", 0.9947260022))?,
            ],
            vertical_density_light_back: vec![
                ScalarCurveEntry::try_from(("08:10", 1.0))?,
            ],
            vertical_density_rim_range: vec![
                ScalarCurveEntry::try_from(("11:32", 1.0))?,
            ],
            custom_color: vec![
                ColorCurveEntry::try_from(("00:15", 3.142, 19.264, 31.347, 1.987))?,
                ColorCurveEntry::try_from(("03:18", 3.142, 19.264, 31.347, 2.084))?,
                ColorCurveEntry::try_from(("03:28", 26.008, 38.008, 50.119, 9.192))?,
                ColorCurveEntry::try_from(("03:43", 77.0, 59.0, 14.0, 9.187))?,
                ColorCurveEntry::try_from(("04:02", 77.0, 59.0, 14.0, 9.187))?,
                ColorCurveEntry::try_from(("04:57", 149.0, 114.0, 25.0, 9.187))?,
                ColorCurveEntry::try_from(("05:41", 233.0, 223.0, 195.0, 5.966))?,
                ColorCurveEntry::try_from(("07:40", 218.713, 213.212, 187.414, 5.371))?,
                ColorCurveEntry::try_from(("11:34", 246.272, 243.531, 231.399, 5.179))?,
                ColorCurveEntry::try_from(("16:49", 244.447, 242.060, 230.879, 5.918))?,
                ColorCurveEntry::try_from(("17:53", 205.904, 206.281, 198.785, 2.72))?,
                ColorCurveEntry::try_from(("20:05", 253.0, 58.0, 1.0, 2.082))?,
                ColorCurveEntry::try_from(("20:17", 57.0, 59.0, 61.0, 2.047))?,
                ColorCurveEntry::try_from(("20:37", 57.0, 59.0, 61.0, 2.047))?,
                ColorCurveEntry::try_from(("23:20", 3.121, 19.243, 31.326, 2.064))?,
            ],
            custom_color_start: vec![
                ScalarCurveEntry::try_from(("02:23", -1.0688883066))?,
                ScalarCurveEntry::try_from(("03:32", 0.2639680505))?,
                ScalarCurveEntry::try_from(("03:35", 0.4961677492))?,
                ScalarCurveEntry::try_from(("03:49", 0.9030510783))?,
                ScalarCurveEntry::try_from(("04:55", 1.0108048916))?,
                ScalarCurveEntry::try_from(("05:12", 0.3837502003))?,
                ScalarCurveEntry::try_from(("05:54", 0.2272521257))?,
                ScalarCurveEntry::try_from(("07:39", 0.2916431427))?,
                ScalarCurveEntry::try_from(("12:01", 0.3027447462))?,
                ScalarCurveEntry::try_from(("16:21", 0.2826163769))?,
                ScalarCurveEntry::try_from(("17:23", 0.3083299398))?,
                ScalarCurveEntry::try_from(("18:27", 1.1753818989))?,
                ScalarCurveEntry::try_from(("19:02", 1.5800062418))?,
                ScalarCurveEntry::try_from(("20:04", 1.6010992527))?,
                ScalarCurveEntry::try_from(("20:20", -0.4585551023))?,
                ScalarCurveEntry::try_from(("20:38", -1.0515730381))?,
                ScalarCurveEntry::try_from(("21:01", -1.0847896338))?,
                ScalarCurveEntry::try_from(("22:14", -1.0914540291))?,
            ],
            custom_color_range: vec![
                ScalarCurveEntry::try_from(("01:04", 3.6806063652))?,
                ScalarCurveEntry::try_from(("04:42", 3.8118515015))?,
                ScalarCurveEntry::try_from(("07:37", 1.9127099514))?,
                ScalarCurveEntry::try_from(("16:04", 1.9300076962))?,
                ScalarCurveEntry::try_from(("19:26", 0.650424242))?,
                ScalarCurveEntry::try_from(("20:39", 2.8878257275))?,
                ScalarCurveEntry::try_from(("20:57", 3.3950767517))?,
            ],
            custom_amount_scale: vec![
                ScalarCurveEntry::try_from(("07:49", 1.0))?,
            ],
            custom_amount_scale_start: vec![
                ScalarCurveEntry::try_from(("10:16", 1.0))?,
            ],
            custom_amount_scale_range: vec![
                ScalarCurveEntry::try_from(("20:24", 1.0))?,
            ],
            aerial_color_front: vec![
                ColorCurveEntry::try_from(("01:33", 0.467, 19.104, 32.557, 12.123))?,
            ],
            aerial_color_middle: vec![
                ColorCurveEntry::try_from(("01:21", 17.363, 16.916, 31.84, 10.044))?,
            ],
            aerial_color_back: vec![
                ColorCurveEntry::try_from(("01:29", 7.0383, 13.51, 26.941, 10.013))?,
            ],
            aerial_final_exp: vec![
                ScalarCurveEntry::try_from(("02:44", 0.2983746529))?,
                ScalarCurveEntry::try_from(("03:24", 0.7329537868))?,
                ScalarCurveEntry::try_from(("03:45", 0.7536482811))?,
                ScalarCurveEntry::try_from(("12:23", 40.0425224304))?,
                ScalarCurveEntry::try_from(("19:25", 1.4706077576))?,
                ScalarCurveEntry::try_from(("21:35", 0.3510617614))?,
                ScalarCurveEntry::try_from(("22:55", 0.2983746529))?,
            ],
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
        },
        fog: FogConfig {
            appear_distance:    vec![ScalarCurveEntry::try_from(("11:29", 4.0))?],
            appear_range:       vec![ScalarCurveEntry::try_from(("13:40", 20.0))?],
            color_front: vec![
                ColorCurveEntry::try_from(("02:58", 51.0, 82.0, 109.0, 16.069))?,
                ColorCurveEntry::try_from(("03:59", 88.0, 128.0, 145.0, 16.049))?,
                ColorCurveEntry::try_from(("07:00", 156.0, 196.0, 250.0, 16.0888))?,
                ColorCurveEntry::try_from(("11:13", 185.0, 185.0, 202.0, 16.069))?,
                ColorCurveEntry::try_from(("14:14", 185.029, 184.971, 201.971, 16.0037))?,
                ColorCurveEntry::try_from(("15:46", 185.0, 185.0, 202.0, 16.069))?,
                ColorCurveEntry::try_from(("17:53", 165.0, 196.0, 228.0, 16.808))?,
                ColorCurveEntry::try_from(("19:57", 112.0, 137.0, 159.0, 15.9527))?,
                ColorCurveEntry::try_from(("21:58", 51.0, 82.0, 109.0, 16.069))?,

            ],
            color_middle: vec![
                ColorCurveEntry::try_from(("02:52", 24.0, 59.0, 89.0, 16.028))?,
                ColorCurveEntry::try_from(("03:30", 23.996, 58.995, 88.995, 16.028))?,
                ColorCurveEntry::try_from(("07:03", 133.0, 167.0, 213.0, 16.047))?,
                ColorCurveEntry::try_from(("12:01", 125.0, 168.0, 226.0, 16.047))?,
                ColorCurveEntry::try_from(("14:15", 126.653, 168.65, 225.611, 15.936))?,
                ColorCurveEntry::try_from(("15:50", 128.0, 169.0, 225.0, 16.047))?,
                ColorCurveEntry::try_from(("18:06", 91.930, 144.2, 196.898, 15.833))?,
                ColorCurveEntry::try_from(("19:57", 48.0, 110.0, 158.0, 15.133))?,
                ColorCurveEntry::try_from(("21:55", 24.0, 59.0, 89.0, 16.028))?,

            ],
            color_back: vec![
                ColorCurveEntry::try_from(("02:39", 24.0, 59.0, 89.0, 15.960))?,
                ColorCurveEntry::try_from(("03:27", 23.994, 58.994, 88.994, 15.961))?,
                ColorCurveEntry::try_from(("06:55", 133.0, 167.0, 213.0, 15.965))?,
                ColorCurveEntry::try_from(("11:44", 126.0, 168.0, 224.0, 15.965))?,
                ColorCurveEntry::try_from(("14:11", 128.779, 169.816, 224.759, 15.977))?,
                ColorCurveEntry::try_from(("15:46", 131.0, 171.0, 225.0, 16.0))?,
                ColorCurveEntry::try_from(("18:00", 113.692, 150.734, 200.796, 16.972))?,
                ColorCurveEntry::try_from(("19:57", 87.0, 120.0, 164.0, 15.977))?,
                ColorCurveEntry::try_from(("21:52", 24.0, 59.0, 89.0, 15.961))?,
            ],
            density: vec![
                ScalarCurveEntry::try_from(("00:43", 0.0031766235))?,
                ScalarCurveEntry::try_from(("02:47", 0.0032830038))?,
                ScalarCurveEntry::try_from(("04:11", 0.0029638633))?,
                ScalarCurveEntry::try_from(("06:58", 0.0015809219))?,
                ScalarCurveEntry::try_from(("11:58", 0.0015277318))?,
                ScalarCurveEntry::try_from(("14:15", 0.0016249666))?,
                ScalarCurveEntry::try_from(("15:58", 0.0015277318))?,
                ScalarCurveEntry::try_from(("19:32", 0.0025383425))?,
            ],
            final_exp: vec![
                ScalarCurveEntry::try_from(("03:01", 1.3500678539))?,
                ScalarCurveEntry::try_from(("06:29", 1.6193209887))?,
                ScalarCurveEntry::try_from(("11:54", 1.5))?,
                ScalarCurveEntry::try_from(("15:59", 1.5472596884))?,
                ScalarCurveEntry::try_from(("18:09", 1.8631025553))?,
                ScalarCurveEntry::try_from(("19:32", 1.4695855379))?,
                ScalarCurveEntry::try_from(("22:23", 1.2382794619))?,
            ],
            distance_clamp: vec![
                ScalarCurveEntry::try_from(("02:00", 8640.1640625))?,
                ScalarCurveEntry::try_from(("03:00", 8640.1640625))?,
                ScalarCurveEntry::try_from(("04:00", 8639.01953125))?,
                ScalarCurveEntry::try_from(("07:00", 8853.5390625))?,
                ScalarCurveEntry::try_from(("12:00", 8853.5390625))?,
                ScalarCurveEntry::try_from(("16:00", 8853.5390625))?,
                ScalarCurveEntry::try_from(("18:00", 8639.0947265625))?,
                ScalarCurveEntry::try_from(("20:00", 8640.01171875))?,
                ScalarCurveEntry::try_from(("22:00", 8640.1640625))?,
            ],
            vertical_offset: vec![
                ScalarCurveEntry::try_from(("02:00", 39.5870819092))?,
                ScalarCurveEntry::try_from(("03:00", 39.8260993958))?,
                ScalarCurveEntry::try_from(("04:00", 39.8260993958))?,
                ScalarCurveEntry::try_from(("07:00", 39.8262519836))?,
                ScalarCurveEntry::try_from(("12:00", 39.8262519836))?,
                ScalarCurveEntry::try_from(("16:00", 39.8262519836))?,
                ScalarCurveEntry::try_from(("17:59", 38.4843521118))?,
                ScalarCurveEntry::try_from(("20:00", 39.8260955811))?,
                ScalarCurveEntry::try_from(("22:00", 39.5870819092))?,
            ],
            vertical_density: vec![
                ScalarCurveEntry::try_from(("02:36", -0.0338166542))?,
                ScalarCurveEntry::try_from(("02:58", -0.0327261612))?,
                ScalarCurveEntry::try_from(("03:59", -0.0357569233))?,
                ScalarCurveEntry::try_from(("07:00", -0.0284394976))?,
                ScalarCurveEntry::try_from(("12:00", -0.0284394976))?,
                ScalarCurveEntry::try_from(("16:00", -0.0333035439))?,
            ],
            vertical_density_light_front: vec![
                ScalarCurveEntry::try_from(("01:22", 0.7069256306))?,
                ScalarCurveEntry::try_from(("03:00", 0.694263339))?,
                ScalarCurveEntry::try_from(("03:58", 0.7875822783))?,
                ScalarCurveEntry::try_from(("07:00", 1.0074540377))?,
                ScalarCurveEntry::try_from(("12:00", 1.0074540377))?,
                ScalarCurveEntry::try_from(("16:00", 1.0074540377))?,
                ScalarCurveEntry::try_from(("18:00", 0.9973887205))?,
                ScalarCurveEntry::try_from(("20:00", 0.7122251987))?,
            ],
            vertical_density_light_back: vec![
                ScalarCurveEntry::try_from(("03:00", 1.0037372112))?,
                ScalarCurveEntry::try_from(("03:58", 1.042681098))?,
                ScalarCurveEntry::try_from(("07:00", 1.0398958921))?,
                ScalarCurveEntry::try_from(("12:00", 1.0398958921))?,
                ScalarCurveEntry::try_from(("16:00", 1.0398958921))?,
                ScalarCurveEntry::try_from(("18:00", 0.9793738127))?,
                ScalarCurveEntry::try_from(("20:00", 0.9917427301))?,
            ],
            vertical_density_rim_range: vec![
                ScalarCurveEntry::try_from(("02:00", 1.0))?,
            ],
            custom_color: vec![
                ColorCurveEntry::try_from(("02:00", 211.000, 229.014, 253.0, 0.587))?,
                ColorCurveEntry::try_from(("03:00", 210.998, 229.012, 253.002, 0.588))?,
                ColorCurveEntry::try_from(("03:57", 167.0, 213.0, 255.0, 3.317))?,
                ColorCurveEntry::try_from(("06:56", 116.0, 166.0, 253.0, 17.0))?,
                ColorCurveEntry::try_from(("12:00", 116.0, 166.0, 253.0, 25.02))?,
                ColorCurveEntry::try_from(("16:00", 116.0, 166.0, 253.0, 20.0))?,
                ColorCurveEntry::try_from(("18:00", 157.081, 214.741, 250.715, 19.28))?,
                ColorCurveEntry::try_from(("20:00", 215.0, 227.0, 240.0, 1.4))?,
                ColorCurveEntry::try_from(("22:00", 211.0, 229.014, 253.0, 0.587))?,
            ],
            custom_color_start: vec![
                ScalarCurveEntry::try_from(("02:00", 0.113895148))?,
                ScalarCurveEntry::try_from(("03:00", 0.113895148))?,
                ScalarCurveEntry::try_from(("04:00", 0.1138959974))?,
                ScalarCurveEntry::try_from(("07:00", 0.1131388471))?,
                ScalarCurveEntry::try_from(("12:00", 0.1131388471))?,
                ScalarCurveEntry::try_from(("16:00", 0.1131388471))?,
                ScalarCurveEntry::try_from(("18:00", 0.1139485091))?,
                ScalarCurveEntry::try_from(("20:00", 0.1139061898))?,
                ScalarCurveEntry::try_from(("22:00", 0.113895148))?,
            ],
            custom_color_range: vec![
                ScalarCurveEntry::try_from(("02:00", 1.3451185226))?,
                ScalarCurveEntry::try_from(("03:00", 1.3451185226))?,
                ScalarCurveEntry::try_from(("04:00", 1.3451100588))?,
                ScalarCurveEntry::try_from(("07:00", 1.3505097628))?,
                ScalarCurveEntry::try_from(("12:00", 1.3505097628))?,
                ScalarCurveEntry::try_from(("16:00", 1.3505097628))?,
                ScalarCurveEntry::try_from(("18:00", 1.3447213173))?,
                ScalarCurveEntry::try_from(("20:00", 1.3450409174))?,
                ScalarCurveEntry::try_from(("22:00", 1.3451185226))?,
            ],
            custom_amount_scale: vec![
                ScalarCurveEntry::try_from(("02:00", 0.9999980927))?,
                ScalarCurveEntry::try_from(("03:00", 0.9999980927))?,
                ScalarCurveEntry::try_from(("04:00", 1.0))?,
                ScalarCurveEntry::try_from(("07:00", 0.9982308149))?,
                ScalarCurveEntry::try_from(("12:00", 0.9982308149))?,
                ScalarCurveEntry::try_from(("16:00", 0.9982308149))?,
                ScalarCurveEntry::try_from(("18:00", 1.0001269579))?,
                ScalarCurveEntry::try_from(("20:00", 1.000023365))?,
                ScalarCurveEntry::try_from(("22:00", 0.9999980927))?,
            ],
            custom_amount_scale_start: vec![
                ScalarCurveEntry::try_from(("02:00", 0.9999973774))?,
                ScalarCurveEntry::try_from(("03:00", 0.9999973774))?,
                ScalarCurveEntry::try_from(("04:00", 1.0))?,
                ScalarCurveEntry::try_from(("07:00", 0.9975706339))?,
                ScalarCurveEntry::try_from(("12:00", 0.9975706339))?,
                ScalarCurveEntry::try_from(("16:00", 0.9975706339))?,
                ScalarCurveEntry::try_from(("18:00", 1.000174284))?,
                ScalarCurveEntry::try_from(("20:00", 1.0000321865))?,
                ScalarCurveEntry::try_from(("22:00", 0.9999973774))?,
            ],
            custom_amount_scale_range: vec![
                ScalarCurveEntry::try_from(("02:00", 1.0))?,
            ],
            aerial_color_front: vec![
                ColorCurveEntry::try_from(("02:00", 4.660, 150.676, 254.976, 1.0))?,
                ColorCurveEntry::try_from(("03:00", 4.660, 150.676, 254.976, 1.0))?,
                ColorCurveEntry::try_from(("04:00", 4.660, 150.676, 254.976, 0.978))?,
                ColorCurveEntry::try_from(("07:00", 36.0, 144.0, 250.0, 0.929))?,
                ColorCurveEntry::try_from(("12:00", 36.0, 144.0, 250.0, 0.929))?,
                ColorCurveEntry::try_from(("16:00", 36.0, 144.0, 250.0, 0.929))?,
                ColorCurveEntry::try_from(("19:18", 4.659, 150.677, 254.977, 0.996))?,
                ColorCurveEntry::try_from(("22:00", 4.660, 150.676, 254.976, 1.0))?,
            ],
            aerial_color_middle: vec![
                ColorCurveEntry::try_from(("02:00", 4.692, 150.688, 254.969, 1.0))?,
                ColorCurveEntry::try_from(("03:00", 4.692, 150.688, 254.969, 1.0))?,
                ColorCurveEntry::try_from(("04:00", 52.366, 179.22, 251.507, 0.972))?,
                ColorCurveEntry::try_from(("07:00", 117.791, 218.368, 246.77, 0.936))?,
                ColorCurveEntry::try_from(("12:00", 117.791, 218.368, 246.77, 0.936))?,
                ColorCurveEntry::try_from(("16:00", 117.791, 218.368, 246.77, 0.936))?,
                ColorCurveEntry::try_from(("19:18", 9.175, 153.386, 254.612, 0.995))?,
                ColorCurveEntry::try_from(("22:00", 4.692, 150.688, 254.969, 1.0))?,
            ],
            aerial_color_back: vec![
                ColorCurveEntry::try_from(("02:00", 5.5, 150.363, 253.69, 1.0))?,
                ColorCurveEntry::try_from(("03:00", 5.5, 150.363, 253.69, 1.0))?,
                ColorCurveEntry::try_from(("04:00", 98.831, 153.214, 167.011, 0.973))?,
                ColorCurveEntry::try_from(("07:00", 235.446, 157.371, 40.134, 0.936))?,
                ColorCurveEntry::try_from(("11:58", 117.791, 218.368, 246.77, 0.936))?,
                ColorCurveEntry::try_from(("16:00", 235.446, 157.371, 40.134, 0.936))?,
                ColorCurveEntry::try_from(("18:55", 10.356, 150.524, 246.72, 0.996))?,
                ColorCurveEntry::try_from(("22:00", 5.5, 150.363, 253.69, 1.0))?,
            ],
            aerial_final_exp: vec![
                ScalarCurveEntry::try_from(("02:00", 3.7810132504))?,
                ScalarCurveEntry::try_from(("03:00", 3.7727270126))?,
                ScalarCurveEntry::try_from(("04:00", 3.2552189827))?,
                ScalarCurveEntry::try_from(("06:57", 2.9996800423))?,
                ScalarCurveEntry::try_from(("12:00", 2.9996800423))?,
                ScalarCurveEntry::try_from(("15:58", 2.9996800423))?,
                ScalarCurveEntry::try_from(("18:00", 1.0571168661))?,
                ScalarCurveEntry::try_from(("20:00", 3.6985480785))?,
                ScalarCurveEntry::try_from(("22:00", 3.7810132504))?,
            ],
        }
    })
}
// ----------------------------------------------------------------------------
