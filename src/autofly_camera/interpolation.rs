// ----------------------------------------------------------------------------
use bevy::math::{Quat, Vec3};

use splines::{Interpolation, Key, Spline};

use super::{CameraRotation, PathKeypoint};
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct PathInterpolation {
    sampling_offset: f32,
    pos: Spline<f32, InterpolatedPosition>,
    rot: Spline<f32, InterpolatedRotation>,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct PathCurveEntry {
    t: f32,
    key_pos: Vec3,
    key_rot: CameraRotation,
    control_pos: Vec3,
    control_rot: CameraRotation,
}
// ----------------------------------------------------------------------------
impl PathInterpolation {
    // ------------------------------------------------------------------------
    pub fn sample(&self, t: f32) -> (Vec3, Quat) {
        let t = (t + self.sampling_offset) % 1.0;

        (
            self.pos.clamped_sample(t).unwrap().to_vec3(),
            self.rot.clamped_sample(t).unwrap().to_quat(),
        )
    }
    // ------------------------------------------------------------------------
    pub fn keypoint_count(&self) -> usize {
        self.pos.len()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl PathCurveEntry {
    // ------------------------------------------------------------------------
    pub(super) fn new(t: f32, keypoint: &PathKeypoint, control: &PathKeypoint) -> Self {
        Self {
            t,
            key_pos: keypoint.pos,
            key_rot: keypoint.rot,
            control_pos: control.pos,
            control_rot: control.rot,
        }
    }
    // ------------------------------------------------------------------------
    fn t(&self) -> f32 {
        self.t
    }
    // ------------------------------------------------------------------------
    fn pos(&self) -> Vec3 {
        self.key_pos
    }
    // ------------------------------------------------------------------------
    fn rot(&self) -> CameraRotation {
        self.key_rot
    }
    // ------------------------------------------------------------------------
    fn pos_control(&self) -> Vec3 {
        self.control_pos
    }
    // ------------------------------------------------------------------------
    fn rot_control(&self) -> CameraRotation {
        self.control_rot
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// local new type to implement all required ops for spline interpolation
// ----------------------------------------------------------------------------
#[derive(Copy, Clone)]
struct InterpolatedPosition(Vec3);
// ----------------------------------------------------------------------------
#[derive(Copy, Clone)]
struct InterpolatedRotation(CameraRotation);
// ----------------------------------------------------------------------------
impl InterpolatedPosition {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn to_vec3(self) -> Vec3 {
        self.0
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl InterpolatedRotation {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn new(pitch: f32, yaw: f32) -> Self {
        Self(CameraRotation { pitch, yaw })
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn to_quat(self) -> Quat {
        self.0.into()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<Vec<PathCurveEntry>> for PathInterpolation {
    // ------------------------------------------------------------------------
    fn from(mut keypoints: Vec<PathCurveEntry>) -> Self {
        use std::f32::consts::PI;
        use Interpolation::{Bezier, Linear};

        match keypoints.len() {
            0 => Self::default(),
            1 => {
                let key = keypoints.pop().unwrap();
                PathInterpolation {
                    sampling_offset: 0.0,
                    pos: Spline::from_vec(vec![Key::new(
                        0.0f32,
                        InterpolatedPosition::from(key.pos()),
                        Linear,
                    )]),
                    rot: Spline::from_vec(vec![Key::new(
                        0.0f32,
                        InterpolatedRotation::from(key.rot()),
                        Linear,
                    )]),
                }
            }
            _ => {
                // TODO remap keys from [t_0..t_n] to [0..t_n-t_0] and add first key
                // at 1.0 to close cycle
                // sampling offset will be used to sample from original range again

                let first = keypoints.first().unwrap();
                let sampling_offset = first.t();

                let pos = keypoints
                    .iter()
                    .map(|k| {
                        Key::new(
                            k.t() - sampling_offset,
                            k.pos().into(),
                            Bezier(k.pos_control().into()),
                        )
                    })
                    .collect::<Vec<_>>();

                // special case for rotation if yaw crosses 360 <-> 0° (pitch
                // is assumed to be clamped (-1.54, 1.54))
                let mut rot: Vec<Key<f32, InterpolatedRotation>> =
                    Vec::with_capacity(keypoints.len());

                rot.push(Key::new(
                    first.t() - sampling_offset,
                    first.rot().into(),
                    Bezier(first.rot_control().into()),
                ));

                // keep and update an offset which moves values if jump to next
                // yaw crosses 360 <-> 0° boundary (either direction) so
                // interpolation can be calculated
                let mut offset_key = 0.0;
                let mut offset_control = 0.0;
                for window in keypoints.as_slice().windows(2) {
                    let (prev, key) = (&window[0], &window[1]);

                    let prev_key_yaw = offset_key + prev.rot().yaw;
                    let mut key_yaw = offset_key + key.rot().yaw;

                    if (key_yaw - prev_key_yaw).abs() > PI {
                        if prev_key_yaw > key_yaw {
                            offset_key += 2.0 * PI;
                            key_yaw += 2.0 * PI;
                        } else {
                            offset_key -= 2.0 * PI;
                            key_yaw -= 2.0 * PI;
                        }
                    }

                    let prev_control_yaw = offset_control + prev.rot_control().yaw;
                    let mut control_yaw = offset_control + key.rot_control().yaw;

                    if (control_yaw - prev_control_yaw).abs() > PI {
                        if prev_control_yaw > control_yaw {
                            offset_control += 2.0 * PI;
                            control_yaw += 2.0 * PI;
                        } else {
                            offset_control -= 2.0 * PI;
                            control_yaw -= 2.0 * PI;
                        }
                    }

                    let key_yaw = CameraRotation::from((key.rot().pitch, key_yaw));
                    let control_yaw = CameraRotation::from((key.rot_control().pitch, control_yaw));

                    rot.push(Key::new(
                        key.t() - sampling_offset,
                        key_yaw.into(),
                        Bezier(control_yaw.into()),
                    ));
                }

                PathInterpolation {
                    sampling_offset: 1.0 - sampling_offset,
                    pos: Spline::from_vec(pos),
                    rot: Spline::from_vec(rot),
                }
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<Quat> for CameraRotation {
    // ------------------------------------------------------------------------
    fn from(q: Quat) -> Self {
        // from https://github.com/mcpar-land/bevy_fly_camera/pull/15/files
        let sinp = 2.0 * (q.w * q.x - q.y * q.z);
        let pitch = sinp.asin();
        let siny_cosp = 2.0 * (q.w * q.y + q.z * q.x);
        let cosy_cosp = 1.0 - 2.0 * (q.x * q.x + q.y * q.y);
        let yaw = siny_cosp.atan2(cosy_cosp);
        Self { pitch, yaw }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<(f32, f32)> for CameraRotation {
    // ------------------------------------------------------------------------
    fn from((pitch, yaw): (f32, f32)) -> Self {
        Self { pitch, yaw }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<CameraRotation> for Quat {
    fn from(c: CameraRotation) -> Self {
        Quat::from_axis_angle(Vec3::Y, c.yaw) * Quat::from_axis_angle(Vec3::X, c.pitch)
    }
}
// ----------------------------------------------------------------------------
impl From<Vec3> for InterpolatedPosition {
    fn from(v: Vec3) -> Self {
        Self(v)
    }
}
// ----------------------------------------------------------------------------
impl From<Quat> for InterpolatedRotation {
    fn from(v: Quat) -> Self {
        Self(v.into())
    }
}
// ----------------------------------------------------------------------------
impl From<CameraRotation> for InterpolatedRotation {
    fn from(v: CameraRotation) -> Self {
        Self(v)
    }
}
// ----------------------------------------------------------------------------
// some std ops implementation
// ----------------------------------------------------------------------------
impl std::ops::Mul<f32> for CameraRotation {
    type Output = Self;
    #[inline(always)]
    fn mul(self, other: f32) -> Self {
        CameraRotation {
            pitch: self.pitch * other,
            yaw: self.yaw * other,
        }
    }
}
impl std::ops::Div<f32> for CameraRotation {
    type Output = Self;
    #[inline(always)]
    fn div(self, other: f32) -> Self {
        CameraRotation {
            pitch: self.pitch / other,
            yaw: self.yaw / other,
        }
    }
}
impl std::ops::Add<CameraRotation> for CameraRotation {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: CameraRotation) -> Self {
        CameraRotation {
            pitch: self.pitch + other.pitch,
            yaw: self.yaw + other.yaw,
        }
    }
}
impl std::ops::Sub<CameraRotation> for CameraRotation {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: CameraRotation) -> Self {
        CameraRotation {
            pitch: self.pitch - other.pitch,
            yaw: self.yaw - other.yaw,
        }
    }
}
// ----------------------------------------------------------------------------
impl std::ops::Mul<f32> for InterpolatedPosition {
    type Output = Self;
    #[inline(always)]
    fn mul(self, other: f32) -> Self {
        Self(self.0 * other)
    }
}
impl std::ops::Div<f32> for InterpolatedPosition {
    type Output = Self;
    #[inline(always)]
    fn div(self, other: f32) -> Self {
        Self(self.0 / other)
    }
}
impl std::ops::Add<InterpolatedPosition> for InterpolatedPosition {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: InterpolatedPosition) -> Self {
        Self(self.0 + other.0)
    }
}
impl std::ops::Sub<InterpolatedPosition> for InterpolatedPosition {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: InterpolatedPosition) -> Self {
        Self(self.0 - other.0)
    }
}
// ----------------------------------------------------------------------------
impl std::ops::Mul<f32> for InterpolatedRotation {
    type Output = Self;
    #[inline(always)]
    fn mul(self, other: f32) -> Self {
        Self(self.0 * other)
    }
}
impl std::ops::Div<f32> for InterpolatedRotation {
    type Output = Self;
    #[inline(always)]
    fn div(self, other: f32) -> Self {
        Self(self.0 / other)
    }
}
impl std::ops::Add<InterpolatedRotation> for InterpolatedRotation {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: InterpolatedRotation) -> Self {
        Self(self.0 + other.0)
    }
}
impl std::ops::Sub<InterpolatedRotation> for InterpolatedRotation {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: InterpolatedRotation) -> Self {
        Self(self.0 - other.0)
    }
}
// ----------------------------------------------------------------------------
splines::impl_Interpolate!(f32, InterpolatedPosition, std::f32::consts::PI);
splines::impl_Interpolate!(f32, InterpolatedRotation, std::f32::consts::PI);
// ----------------------------------------------------------------------------
// Default impl
// ----------------------------------------------------------------------------
impl Default for PathInterpolation {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        use Interpolation::Linear;
        Self {
            sampling_offset: 0.0,
            pos: Spline::from_vec(vec![Key::new(
                0.0f32,
                InterpolatedPosition::new(1.0, 1.0, 1.0),
                Linear,
            )]),
            rot: Spline::from_vec(vec![Key::new(
                0.0f32,
                InterpolatedRotation::new(0.0, 0.0),
                Linear,
            )]),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
