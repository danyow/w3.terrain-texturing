// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::atmosphere::AtmosphereMat;
use crate::shapes::XZGrid;
use crate::terrain_render::EnvironmentData;

use super::{Angle, DayNightCycle};
// ----------------------------------------------------------------------------
// Marker for updating the position of the light, not needed unless we have multiple lights
#[derive(Component)]
pub struct Sun;
// ----------------------------------------------------------------------------
pub struct SunPositionSettings {
    yaw: Angle,  // base rotation
    tilt: Angle, // axial tilt
    height: u16,
    show_dbg_mesh: bool,
}
// ----------------------------------------------------------------------------
impl SunPositionSettings {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn plane_yaw(&self) -> Angle {
        self.yaw
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn plane_tilt(&self) -> Angle {
        self.tilt
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn plane_height(&self) -> u16 {
        self.height
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn show_debug_mesh(&self) -> bool {
        self.show_dbg_mesh
    }
    // ------------------------------------------------------------------------
    pub fn set_plane_tilt(&mut self, tilt: u16) {
        self.tilt = Angle::new(tilt);
    }
    // ------------------------------------------------------------------------
    pub fn set_plane_yaw(&mut self, yaw: u16) {
        self.yaw = Angle::new(yaw);
    }
    // ------------------------------------------------------------------------
    pub fn set_plane_height(&mut self, height: u16) {
        self.height = height;
    }
    // ------------------------------------------------------------------------
    pub fn toggle_debug_mesh(&mut self) {
        self.show_dbg_mesh = !self.show_dbg_mesh;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// helper components
// ----------------------------------------------------------------------------
#[derive(Component, Default)]
pub(super) struct SunPlane {
    visualize: bool,
}
#[derive(Component)]
pub(super) struct SunDebugMesh;
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::type_complexity)]
pub(super) fn update_sun_position(
    day_night_cycle: Res<DayNightCycle>,
    settings: Res<SunPositionSettings>,
    mut sky_mat: ResMut<AtmosphereMat>,
    mut environment: ResMut<EnvironmentData>,
    mut query: QuerySet<(
        QueryState<(&mut Transform, &mut SunPlane)>,
        QueryState<&mut Visibility, With<SunDebugMesh>>,
    )>,
    sun: Query<&mut GlobalTransform, With<Sun>>,
) {
    const PLANE_HEIGHT_SCALE: f32 = 50.0;

    if settings.is_changed() || day_night_cycle.is_changed() {
        let sun_daytime = day_night_cycle.time_of_day().to_radians();
        let sun_plane_tilt = settings.plane_tilt().as_radians();
        let sun_plane_yaw = settings.plane_yaw().as_radians();

        if let Ok((mut transform, mut sunplane)) = query.q0().get_single_mut() {
            let height_adjustment = PLANE_HEIGHT_SCALE * settings.plane_height() as f32;

            transform.rotation =
                Quat::from_euler(EulerRot::YXZ, sun_plane_yaw, sun_plane_tilt, sun_daytime);
            transform.translation = Vec3::new(0.0, height_adjustment, 0.0);

            // flip visibility only if it really changed
            if sunplane.visualize != settings.show_debug_mesh() {
                sunplane.visualize = settings.show_debug_mesh();

                for mut dbg_mesh_visibility in query.q1().iter_mut() {
                    dbg_mesh_visibility.is_visible = settings.show_debug_mesh();
                }
            }
        }

        if let Ok(sun_transform) = sun.get_single() {
            sky_mat.set_sun_position(sun_transform.translation);
            environment.sun.direction = -sun_transform.translation;
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn setup_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    daylight_cycle: Res<DayNightCycle>,
    settings: Res<SunPositionSettings>,
) {
    info!("initialize sun setup");

    let sun_daytime = daylight_cycle.time_of_day().to_radians();
    let sun_plane_tilt = settings.plane_tilt().as_radians();

    let sun_size = 80.0;
    let sun_distance = 10000.0;
    let plane_size = 250.0;
    let dbg_thickness = 0.5;

    fn new_material(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color,
            emissive: color,
            unlit: true,
            ..Default::default()
        }
    }

    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(1.0, 1.0, 0.6),
        emissive: Color::rgb(1.0, 0.5, 0.5),
        unlit: true,
        ..Default::default()
    });

    let sun_axis_mat = sun_mat.clone();

    let sun_mesh = MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: sun_size,
            subdivisions: 5,
        })),
        material: sun_mat,
        transform: Transform::from_xyz(0.0, -sun_distance, 0.0),
        ..Default::default()
    };

    let plane_axis_x_mesh = MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            plane_size,
            dbg_thickness,
            dbg_thickness,
        ))),
        material: materials.add(new_material(Color::RED)),
        ..Default::default()
    };

    let plane_axis_y_mesh = MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            dbg_thickness,
            sun_distance,
            dbg_thickness,
        ))),
        material: sun_axis_mat,
        transform: Transform::from_xyz(0.0, -sun_distance * 0.5, 0.0),
        ..Default::default()
    };

    let plane_axis_z_mesh = MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            dbg_thickness,
            dbg_thickness,
            plane_size,
        ))),
        material: materials.add(new_material(Color::BLUE)),
        ..Default::default()
    };

    let plane_grid_mesh = MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(XZGrid::new(plane_size, plane_size / 20.0))),
        material: materials.add(new_material(Color::GREEN)),
        // TODO scale depending on map size?
        // transform: Transform::from_scale(Vec3::ONE * 2.0),
        ..Default::default()
    };

    commands
        .spawn_bundle((
            GlobalTransform::default(),
            Transform::from_rotation(Quat::from_euler(
                EulerRot::XZY,
                sun_plane_tilt,
                sun_daytime,
                0.0,
            )),
        ))
        .insert(SunPlane { visualize: true })
        .with_children(|parent| {
            parent.spawn_bundle(plane_grid_mesh).insert(SunDebugMesh);
            parent.spawn_bundle(plane_axis_x_mesh).insert(SunDebugMesh);
            parent.spawn_bundle(plane_axis_z_mesh).insert(SunDebugMesh);
            parent.spawn_bundle(plane_axis_y_mesh).insert(SunDebugMesh);
            parent
                .spawn_bundle(sun_mesh)
                .insert(SunDebugMesh)
                .insert(Sun);
        });
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
impl Default for SunPositionSettings {
    fn default() -> Self {
        Self {
            yaw: Angle::new(0),
            tilt: Angle::new(23), // earth axial tilt ~23.437°
            height: 0,
            show_dbg_mesh: false,
        }
    }
}
// ----------------------------------------------------------------------------
