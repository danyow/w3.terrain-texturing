// ----------------------------------------------------------------------------
use bevy::{
    core::Time,
    ecs::schedule::StateData,
    math::{Quat, Vec3},
    prelude::*,
    render::camera::Camera3d,
};

use self::{
    interpolation::{PathCurveEntry, PathInterpolation},
    visualization::VisualizedPathInterpolation,
};
// ----------------------------------------------------------------------------
mod interpolation;
mod shapes;
mod visualization;
// ----------------------------------------------------------------------------
pub struct AutoFlyCameraPlugin;
// ----------------------------------------------------------------------------
pub struct CameraPathsCollection {
    selected: usize,
    paths: Vec<CameraPath>,
    default_path: CameraPath,
}
// ----------------------------------------------------------------------------
impl AutoFlyCameraPlugin {
    // ------------------------------------------------------------------------
    pub fn setup_autofly_path<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(setup_flypath)
    }
    // ------------------------------------------------------------------------
    pub fn active_autofly_camera<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state)
            .with_system(autofly_cam)
            .with_system(autofly_hotkeys)
    }
    // ------------------------------------------------------------------------
    pub fn stop_auto_fly<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_exit(state).with_system(stop_autofly)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for AutoFlyCameraPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraPathsCollection>()
            .init_resource::<AutoFlyCameraPath>()
            .init_resource::<CameraPath>()
            .init_resource::<PathInterpolation>()
            .add_plugin(visualization::CameraPathVisualizationPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CameraPathsCollection {
    // ------------------------------------------------------------------------
    pub fn select(&mut self, id: usize) {
        self.selected = id;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn setup_flypath(
    time: Res<Time>,
    campaths: Res<CameraPathsCollection>,
    query: Query<&Transform, With<Camera3d>>,
    mut autofly: ResMut<AutoFlyCameraPath>,
) {
    let cam = query.get_single().expect("Camera3d missing");

    autofly.set_path_from_current_cam(
        &*time,
        (cam.translation, cam.rotation),
        campaths.get(campaths.selected()),
    );
}
// ----------------------------------------------------------------------------
fn stop_autofly(
    mut autofly: ResMut<AutoFlyCameraPath>,
    mut visualization: ResMut<VisualizedPathInterpolation>,
) {
    autofly.active = false;
    if visualization.as_ref().is_active() {
        visualization.remove();
    }
}
// ----------------------------------------------------------------------------
fn autofly_cam(
    time: Res<Time>,
    autofly: Res<AutoFlyCameraPath>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    if autofly.active {
        if let Ok(mut cam) = query.get_single_mut() {
            let t = (time.seconds_since_startup() - autofly.startup) as f32 / autofly.duration;

            let (pos, rot) = autofly.path.sample(t);
            cam.translation = pos;
            cam.rotation = rot;
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::single_match)]
fn autofly_hotkeys(
    keys: Res<Input<KeyCode>>,
    autofly: Res<AutoFlyCameraPath>,
    mut visualization: ResMut<VisualizedPathInterpolation>,
) {
    for key in keys.get_just_pressed() {
        match key {
            KeyCode::V => {
                if visualization.as_ref().is_active() {
                    visualization.remove();
                } else {
                    visualization.set(autofly.path.clone());
                }
            }
            _ => (),
        }
    }
}
// ----------------------------------------------------------------------------
// internal types
// ----------------------------------------------------------------------------
#[derive(Default)]
struct AutoFlyCameraPath {
    active: bool,
    startup: f64,
    duration: f32,
    path: PathInterpolation,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq)]
struct CameraRotation {
    pitch: f32,
    yaw: f32,
}
// ----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
struct PathKeypoint {
    pos: Vec3,
    rot: CameraRotation,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
struct CameraPath {
    duration: f32,
    keypoints: Vec<PathKeypoint>,
    control_points: Vec<PathKeypoint>,
}
// ----------------------------------------------------------------------------
trait InterpolatedPath {
    fn duration(&self) -> f32;
    fn interpolation(&self) -> PathInterpolation;
    fn interpolation_with_start(&self, start_pos: Vec3, start_rotation: Quat) -> PathInterpolation;
}
// ----------------------------------------------------------------------------
impl CameraPathsCollection {
    // ------------------------------------------------------------------------
    fn get(&self, id: usize) -> &CameraPath {
        // TODO as hash/asset + handle
        self.paths.get(id).unwrap_or(&self.default_path)
    }
    // ------------------------------------------------------------------------
    fn selected(&self) -> usize {
        self.selected
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CameraPath {
    // ------------------------------------------------------------------------
    fn set_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'a> InterpolatedPath for &'a CameraPath {
    // ------------------------------------------------------------------------
    fn duration(&self) -> f32 {
        self.duration
    }
    // ------------------------------------------------------------------------
    fn interpolation(&self) -> PathInterpolation {
        let k = self.keypoints.len();

        let keypoints = self
            .keypoints
            .iter()
            .zip(self.control_points.iter())
            .enumerate()
            .map(|(i, (key, control))| PathCurveEntry::new(i as f32 / (k - 1) as f32, key, control))
            .collect::<Vec<_>>();

        PathInterpolation::from(keypoints)
    }
    // ------------------------------------------------------------------------
    fn interpolation_with_start(&self, start_pos: Vec3, start_rotation: Quat) -> PathInterpolation {
        let k = self.keypoints.len() + 1;

        let mut keypoints = Vec::with_capacity(k);

        keypoints.push(PathCurveEntry::new(
            0.0,
            &PathKeypoint::from((start_pos, start_rotation)),
            &PathKeypoint::from((start_pos, start_rotation)),
        ));

        for (i, (key, control)) in self
            .keypoints
            .iter()
            .zip(self.control_points.iter())
            .enumerate()
        {
            keypoints.push(PathCurveEntry::new(
                (i + 1) as f32 / (k - 1) as f32,
                key,
                control,
            ));
        }

        PathInterpolation::from(keypoints)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AutoFlyCameraPath {
    // ------------------------------------------------------------------------
    fn set_path_from_current_cam<P: InterpolatedPath>(
        &mut self,
        time: &Time,
        (start_pos, start_rotation): (Vec3, Quat),
        path: P,
    ) {
        self.path = path.interpolation_with_start(start_pos, start_rotation);
        self.startup = time.seconds_since_startup();
        self.duration = path.duration();
        self.active = true;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl PathKeypoint {
    // ------------------------------------------------------------------------
    fn new<P: Into<Vec3>, R: Into<CameraRotation>>(pos: P, rot: R) -> Self {
        Self {
            pos: pos.into(),
            rot: rot.into(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<Vec<(PathKeypoint, PathKeypoint)>> for CameraPath {
    // ------------------------------------------------------------------------
    fn from(mut keys: Vec<(PathKeypoint, PathKeypoint)>) -> Self {
        let (keypoints, control_points) = keys.drain(..).unzip();
        Self {
            duration: keys.len() as f32,
            keypoints,
            control_points,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<(Vec3, Quat)> for PathKeypoint {
    fn from((pos, rot): (Vec3, Quat)) -> Self {
        Self {
            pos,
            rot: CameraRotation::from(rot),
        }
    }
}
// ----------------------------------------------------------------------------
// default impl
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl Default for CameraPathsCollection {
    fn default() -> Self {
        Self {
            selected: 0,
            paths: vec![
                CameraPath::from(
                    vec![
                    // Prolog flyby
(PathKeypoint::new((255.91779, 10.622536, -76.13155), (-0.10171645, 1.1042833)), PathKeypoint::new((177.8572, 17.39462, -118.93), (0.058383606, 0.78918463))),
(PathKeypoint::new((-63.736633, 26.932966, -389.8001), (-0.19028237, 1.2149942)), PathKeypoint::new((-209.25946, 33.00782, -544.8217), (-0.13918665, 2.17389))),
(PathKeypoint::new((-442.76254, 29.65642, -490.2772), (-0.2090176, 3.107241)), PathKeypoint::new((-534.4407, 34.387096, -389.53348), (-0.24648722, 3.125976))),
(PathKeypoint::new((-300.4297, 46.70152, 409.05606), (-0.15451533, -2.097826)), PathKeypoint::new((-138.41705, 44.351563, 623.7485), (-0.14599925, -1.0282187))),
(PathKeypoint::new((63.607903, 30.400412, 349.84894), (-0.17665695, -0.27541322)), PathKeypoint::new((85.48069, 21.930632, 192.8483), (-0.16814062, -1.4063339))),
(PathKeypoint::new((353.05606, 14.783169, 217.19731), (-0.09490356, -1.6992819)), PathKeypoint::new((465.42148, 26.03579, 285.06723), (-0.14088984, -1.0725076))),
(PathKeypoint::new((504.9525, 18.048256, 235.3283), (-0.13486393, -0.5417764)), PathKeypoint::new((525.9277, 15.619314, 220.25232), (-0.27452505, -0.8296174))),
                    ]
                ).set_duration(30.0),
                CameraPath::from(
                    vec![
                    // KM flyby
(PathKeypoint::new((56.97353, 543.30927, 3241.5125), (-0.11470131, -0.12294897)), PathKeypoint::new((169.61308, 545.84686, 2882.5461), (-0.012974373, 0.123259805))),
(PathKeypoint::new((220.15457, 276.44788, 1460.1501), (0.16689055, 0.7498375)), PathKeypoint::new((217.09502, 246.26442, 1053.0331), (0.26124647, 1.1950774))),
(PathKeypoint::new((32.115387, 170.37816, 800.5958), (0.20522296, 0.6540091)), PathKeypoint::new((3.7818117, 155.6384, 695.66534), (0.12413589, 0.7350955))),
(PathKeypoint::new((11.072268, 153.73221, 523.7364), (0.06663814, 1.0078404)), PathKeypoint::new((-23.39103, 158.82796, 459.71487), (-0.2916172, 0.56997275))),
(PathKeypoint::new((-170.25793, 202.21704, 208.69266), (0.022409113, -0.16717865)), PathKeypoint::new((-325.658, 219.9963, 17.369003), (-0.0041285385, -0.8379863))),
(PathKeypoint::new((-69.9166, 195.5186, -3.2402902), (-0.28129748, -0.52543527)), PathKeypoint::new((-18.306587, 187.87553, -90.28981), (-0.21348023, 0.4181199))),
(PathKeypoint::new((48.562737, 64.96414, -467.42825), (0.5620042, -0.058079917)), PathKeypoint::new((84.31649, 67.725204, -795.8491), (0.6268747, 0.9842528))),
(PathKeypoint::new((-155.89055, 222.1669, -1092.7212), (0.49271277, 2.1622238)), PathKeypoint::new((-509.76254, 225.56508, -1075.3801), (0.36444816, 2.2816348))),
(PathKeypoint::new((-349.52988, 198.44754, -512.10626), (-0.66461676, -1.8829784)), PathKeypoint::new((-123.21274, 38.767982, -488.5217), (-0.66461676, -1.8829784))),
(PathKeypoint::new((7.9420624, 49.093983, -351.64032), (0.42784193, 3.005515)), PathKeypoint::new((-23.827808, 98.07367, -241.1435), (0.42784193, 3.005515))),
(PathKeypoint::new((-188.58658, 209.00455, -121.90019), (0.37034306, 1.9499166)), PathKeypoint::new((-300.6122, 223.99196, 55.972622), (0.37034306, 1.9499166))),
(PathKeypoint::new((-1030.9341, 510.7023, 393.7667), (-0.20905852, -2.920887)), PathKeypoint::new((-1007.3239, 510.7023, 501.26663), (-0.23264776, -1.8638119))),
(PathKeypoint::new((-479.86066, 510.7023, 1148.5018), (-0.14124133, -0.8539164)), PathKeypoint::new((-314.99176, 510.7023, 1169.6135), (-0.12649775, -0.82442635))),
(PathKeypoint::new((-215.36928, 481.96423, 1021.52295), (0.032392498, -0.08232771)), PathKeypoint::new((-215.36928, 481.96423, 1021.52295), (0.032392498, -0.08232771))),
                    ]
                ).set_duration(90.0)
            ],
            default_path:
                CameraPath::from(
                    vec![
(PathKeypoint::new((-141.28, 196.04, 274.18), (-0.34, -0.55)), PathKeypoint::new((-147.65, 222.48, 240.78), (-0.29, -0.82))),
(PathKeypoint::new((-139.38, 192.10, -21.93), (-0.07, -1.14)), PathKeypoint::new((-122.76, 202.12, -34.37), (-0.31, -0.49))),
                    ]
                ).set_duration(10.0)
        }
    }
}
// ----------------------------------------------------------------------------
