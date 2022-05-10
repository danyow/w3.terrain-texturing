// ----------------------------------------------------------------------------
use bevy::{math::Vec3, prelude::Color};

use splines::{Interpolation, Key, Spline};

use super::{ColorCurveEntry, ScalarCurveEntry, TimeOfDay};
// ----------------------------------------------------------------------------
pub struct ScalarInterpolation {
    sampling_offset: f32,
    value: Spline<f32, f32>,
}
// ----------------------------------------------------------------------------
pub struct ColorInterpolation {
    sampling_offset: f32,
    color: Spline<f32, InterpolatedColor>,
    intensity: Spline<f32, f32>,
}
// ----------------------------------------------------------------------------
impl ScalarInterpolation {
    // ------------------------------------------------------------------------
    pub fn sample(&self, time: &TimeOfDay) -> f32 {
        let time = (time.normalized() + self.sampling_offset) % 1.0;
        self.value.clamped_sample(time).unwrap()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ColorInterpolation {
    // ------------------------------------------------------------------------
    pub fn sample(&self, time: &TimeOfDay) -> Color {
        let time = (time.normalized() + self.sampling_offset) % 1.0;

        let col = self.color.clamped_sample(time).unwrap();
        let intensity = self.intensity.clamped_sample(time).unwrap();

        let col = col * intensity;
        Color::rgb_linear(col.0.x, col.0.y, col.0.z)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<Vec<ScalarCurveEntry>> for ScalarInterpolation {
    // ------------------------------------------------------------------------
    fn from(mut keypoints: Vec<ScalarCurveEntry>) -> Self {
        use Interpolation::{Cosine, Linear};

        match keypoints.len() {
            0 => Self {
                sampling_offset: 0.0,
                value: Spline::from_vec(vec![Key::new(0.0f32, 1.0, Linear)]),
            },
            1 => {
                let key = keypoints.pop().unwrap();
                Self {
                    sampling_offset: 0.0,
                    value: Spline::from_vec(vec![Key::new(0.0f32, key.value(), Linear)]),
                }
            }
            _ => {
                let first = keypoints.first().unwrap();
                let sampling_offset = first.time().normalized();

                // remap keys from [t_0..t_n] to [0..t_n-t_0] and add first key
                // at 1.0 to close cycle
                // sampling offset will be used to sample from original range again
                let mut values = keypoints
                    .iter()
                    .map(|k| Key::new(k.time().normalized() - sampling_offset, k.value(), Cosine))
                    .collect::<Vec<_>>();

                values.push(Key::new(1.0, first.value(), Cosine));

                Self {
                    sampling_offset: 1.0 - sampling_offset,
                    value: Spline::from_vec(values),
                }
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<Vec<ColorCurveEntry>> for ColorInterpolation {
    // ------------------------------------------------------------------------
    fn from(mut keypoints: Vec<ColorCurveEntry>) -> Self {
        use Interpolation::{Cosine, Linear};

        match keypoints.len() {
            0 => ColorInterpolation {
                sampling_offset: 0.0,
                color: Spline::from_vec(vec![Key::new(
                    0.0f32,
                    InterpolatedColor::rgb(1.0, 1.0, 1.0),
                    Linear,
                )]),
                intensity: Spline::from_vec(vec![Key::new(0.0f32, 1.0, Linear)]),
            },
            1 => {
                let key = keypoints.pop().unwrap();
                ColorInterpolation {
                    sampling_offset: 0.0,
                    color: Spline::from_vec(vec![Key::new(
                        0.0f32,
                        InterpolatedColor::from(key.color()),
                        Linear,
                    )]),
                    intensity: Spline::from_vec(vec![Key::new(0.0f32, key.intensity(), Linear)]),
                }
            }
            _ => {
                let first = keypoints.first().unwrap();
                let sampling_offset = first.time().normalized();

                // remap keys from [t_0..t_n] to [0..t_n-t_0] and add first key
                // at 1.0 to close cycle
                // sampling offset will be used to sample from original range again
                let mut color = keypoints
                    .iter()
                    .map(|k| {
                        Key::new(
                            k.time().normalized() - sampling_offset,
                            k.color().into(),
                            Cosine,
                        )
                    })
                    .collect::<Vec<_>>();

                let mut intensity = keypoints
                    .iter()
                    .map(|k| {
                        Key::new(
                            k.time().normalized() - sampling_offset,
                            k.intensity(),
                            Cosine,
                        )
                    })
                    .collect::<Vec<_>>();

                color.push(Key::new(1.0, first.color().into(), Cosine));
                intensity.push(Key::new(1.0, first.intensity(), Cosine));

                ColorInterpolation {
                    sampling_offset: 1.0 - sampling_offset,
                    color: Spline::from_vec(color),
                    intensity: Spline::from_vec(intensity),
                }
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'a> From<&'a Color> for InterpolatedColor {
    fn from(c: &'a Color) -> Self {
        InterpolatedColor(Vec3::from_slice(&c.as_linear_rgba_f32()))
    }
}
// ----------------------------------------------------------------------------
// local new type to implement all required ops for spline interpolation
// ----------------------------------------------------------------------------
#[derive(Copy, Clone)]
struct InterpolatedColor(Vec3);

impl InterpolatedColor {
    fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self(Vec3::new(r, g, b))
    }
}

impl std::ops::Mul<f32> for InterpolatedColor {
    type Output = Self;
    #[inline(always)]
    fn mul(self, other: f32) -> Self {
        Self(self.0 * other)
    }
}

impl std::ops::Div<f32> for InterpolatedColor {
    type Output = Self;
    #[inline(always)]
    fn div(self, other: f32) -> Self {
        Self(self.0 / other)
    }
}

impl std::ops::Add<InterpolatedColor> for InterpolatedColor {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: InterpolatedColor) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Sub<InterpolatedColor> for InterpolatedColor {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: InterpolatedColor) -> Self {
        Self(self.0 - other.0)
    }
}

splines::impl_Interpolate!(f32, InterpolatedColor, std::f32::consts::PI);
// ----------------------------------------------------------------------------
