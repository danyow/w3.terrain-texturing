// ----------------------------------------------------------------------------
// ported from bevy_atmosphere:
//  https://github.com/JonahPlusPlus/bevy_atmosphere
//  by Jonah Henriksson
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
//! A procedural sky plugin for bevy
//!
//! ## Example
//! ```
//! use bevy::prelude::*;
//! use bevy_atmosphere::*;
//!
//! fn main() {
//!     App::new()
//!             // Default Earth sky
//!         .insert_resource(bevy_atmosphere::AtmosphereMat::default())
//!         .add_plugins(DefaultPlugins)
//!             // Set to false since we aren't changing the sky's appearance
//!         .add_plugin(bevy_atmosphere::AtmospherePlugin { dynamic: false })
//!         .add_startup_system(setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn_bundle(PerspectiveCameraBundle::default());
//! }
//! ```
// ----------------------------------------------------------------------------
/// Sets up the atmosphere and the systems that control it
///
/// Follows the first camera it finds
#[derive(Default)]
pub struct AtmospherePlugin {
    /// If set to `true`, whenever the [`AtmosphereMat`](crate::AtmosphereMat) resource (if it
    /// exists) is changed, the sky is updated
    ///
    /// If set to `false`, whenever the sky needs to be updated, it will have to be done manually
    /// through a system
    ///
    /// To update the sky manually in a system, you will need the [`AtmosphereMat`](crate::AtmosphereMat)
    /// resource, a [`Handle`](bevy::asset::Handle) to the [`AtmosphereMat`](crate::AtmosphereMat)
    /// used and the [`Assets`](bevy::asset::Assets) that stores the [`AtmosphereMat`](crate::AtmosphereMat)
    /// ### Example
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_atmosphere::AtmosphereMat;
    /// use std::ops::Deref;
    ///
    /// fn atmosphere_dynamic_sky(
    ///     config: Res<AtmosphereMat>,
    ///     sky_mat_query: Query<&Handle<AtmosphereMat>>,
    ///     mut sky_materials: ResMut<Assets<AtmosphereMat>>,
    /// ) {
    ///     if config.is_changed() {
    ///         if let Some(sky_mat_handle) = sky_mat_query.iter().next() {
    ///             if let Some(sky_mat) = sky_materials.get_mut(sky_mat_handle) {
    ///                 *sky_mat = config.deref().clone();
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub dynamic: bool,
}
// ----------------------------------------------------------------------------
pub use material::AtmosphereMat;
// ----------------------------------------------------------------------------
use std::ops::Deref;

use bevy::{
    pbr::{MaterialMeshBundle, MaterialPlugin},
    prelude::{
        shape, AddAsset, App, Assets, Camera, Commands, Handle, Mesh,
        ParallelSystemDescriptorCoercion, Plugin, Query, Res, ResMut, Transform, With, Without,
    },
};
// ----------------------------------------------------------------------------
mod material;
// ----------------------------------------------------------------------------
impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AtmosphereMat>()
            .add_plugin(MaterialPlugin::<AtmosphereMat>::default())
            .add_startup_system(atmosphere_add_sky_sphere)
            .add_system(atmosphere_sky_follow);
        if self.dynamic {
            app.add_system(atmosphere_dynamic_sky.after("sun_position_update"));
        }
    }
}
// ----------------------------------------------------------------------------
fn atmosphere_add_sky_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<AtmosphereMat>>,
    config: Option<Res<AtmosphereMat>>,
) {
    let sky_material = match config {
        None => AtmosphereMat::default(),
        Some(c) => c.deref().clone(),
    };

    let sky_material = sky_materials.add(sky_material);

    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            // FIXME -10.0 this was flickering with big terrain meshes
            //radius: -10.0
            radius: -65536.0,
            subdivisions: 2,
        })),
        material: sky_material,
        ..Default::default()
    });
}
// ----------------------------------------------------------------------------
fn atmosphere_sky_follow(
    camera_transform_query: Query<&Transform, (With<Camera>, Without<Handle<AtmosphereMat>>)>,
    mut sky_transform_query: Query<&mut Transform, With<Handle<AtmosphereMat>>>,
) {
    if let Some(camera_transform) = camera_transform_query.iter().next() {
        if let Some(mut sky_transform) = sky_transform_query.iter_mut().next() {
            sky_transform.translation = camera_transform.translation;
        }
    }
}
// ----------------------------------------------------------------------------
fn atmosphere_dynamic_sky(
    config: Res<AtmosphereMat>,
    sky_mat_query: Query<&Handle<AtmosphereMat>>,
    mut sky_materials: ResMut<Assets<AtmosphereMat>>,
) {
    if config.is_changed() {
        if let Some(sky_mat_handle) = sky_mat_query.iter().next() {
            if let Some(sky_mat) = sky_materials.get_mut(sky_mat_handle) {
                *sky_mat = config.deref().clone();
            }
        }
    }
}
// ----------------------------------------------------------------------------
