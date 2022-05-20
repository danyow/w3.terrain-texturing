// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// Nearly identical to bevy RenderAsset but:
//    instead of immutable extract_asset it allows a mutable access to asset
//    *after* the assets pending_update signals that extraction is required.
//    this enables to move data into render world (e.g. via take from an option)
//    instead of cloning it.
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
use bevy::asset::Asset;
use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use bevy::prelude::*;
use bevy::render::render_asset::PrepareAssetError;
use bevy::render::{RenderApp, RenderStage};
use bevy::utils::{HashMap, HashSet};
use std::marker::PhantomData;

/// Describes how an asset gets extracted and prepared for rendering.
///
/// In the [`RenderStage::Extract`](crate::RenderStage::Extract) step the asset is transferred
/// from the "app world" into the "render world".
/// Therefore it is converted into a [`MutRenderAsset::ExtractedAsset`], which may be the same type
/// as the render asset itself.
///
/// After that in the [`RenderStage::Prepare`](crate::RenderStage::Prepare) step the extracted asset
/// is transformed into its GPU-representation of type [`MutRenderAsset::PreparedAsset`].
pub trait MutRenderAsset: Asset {
    /// The representation of the the asset in the "render world".
    type ExtractedAsset: Send + Sync + 'static;
    /// The GPU-representation of the the asset.
    type PreparedAsset: Send + Sync + 'static;
    /// Specifies all ECS data required by [`MutRenderAsset::prepare_asset`].
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) [`SystemParam`].
    type Param: SystemParam;

    fn pending_update(&self) -> bool;
    /// Converts the asset into a [`MutRenderAsset::ExtractedAsset`].
    fn extract_asset(&mut self) -> Self::ExtractedAsset;
    /// Prepares the `extracted asset` for the GPU by transforming it into
    /// a [`MutRenderAsset::PreparedAsset`]. Therefore ECS data may be accessed via the `param`.
    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>>;
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`MutRenderAssets`] resource.
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) and
/// [`RenderStage::Prepare`](crate::RenderStage::Prepare) steps for the specified [`MutRenderAsset`].
pub struct MutRenderAssetPlugin<A: MutRenderAsset>(PhantomData<fn() -> A>);

impl<A: MutRenderAsset> Default for MutRenderAssetPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: MutRenderAsset> Plugin for MutRenderAssetPlugin<A> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedMutAssets<A>>()
                .init_resource::<MutRenderAssets<A>>()
                .init_resource::<PrepareNextFrameAssets<A>>()
                .add_system_to_stage(
                    RenderStage::Extract,
                    extract_render_asset::<A>.label("extract_mut_render_asset"),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_assets::<A>.label("prepare_mut_render_asset"),
                );
        }
    }
}

/// Temporarily stores the extracted and removed assets of the current frame.
pub struct ExtractedMutAssets<A: MutRenderAsset> {
    extracted: Vec<(Handle<A>, A::ExtractedAsset)>,
    removed: Vec<Handle<A>>,
}

impl<A: MutRenderAsset> Default for ExtractedMutAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`MutRenderAsset::PreparedAssets`](MutRenderAsset::PreparedAsset))
/// of [`MutRenderAssets`](MutRenderAsset) as long as they exist.
pub type MutRenderAssets<A> = HashMap<Handle<A>, <A as MutRenderAsset>::PreparedAsset>;

/// This system extracts all crated or modified assets of the corresponding [`MutRenderAsset`] type
/// into the "render world".
fn extract_render_asset<A: MutRenderAsset>(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<A>>,
    mut assets: ResMut<Assets<A>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                changed_assets.insert(handle);
            }
            AssetEvent::Modified { handle } => {
                changed_assets.insert(handle);
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        // non mutable access to query if any update is pending to prevent
        // mut-access -> modified event
        if assets
            .get(handle)
            .map(|a| a.pending_update())
            .unwrap_or(false)
        {
            // thiw will retrigger a modified event put pedning_update is supposed
            // to catch this
            if let Some(asset) = assets.get_mut(handle) {
                extracted_assets.push((handle.clone_weak(), asset.extract_asset()));
            }
        }
    }

    commands.insert_resource(ExtractedMutAssets {
        extracted: extracted_assets,
        removed,
    });
}

// TODO: consider storing inside system?
/// All assets that should be prepared next frame.
pub struct PrepareNextFrameAssets<A: MutRenderAsset> {
    assets: Vec<(Handle<A>, A::ExtractedAsset)>,
}

impl<A: MutRenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system prepares all assets of the corresponding [`MutRenderAsset`] type
/// which where extracted this frame for the GPU.
fn prepare_assets<R: MutRenderAsset>(
    mut extracted_assets: ResMut<ExtractedMutAssets<R>>,
    mut render_assets: ResMut<MutRenderAssets<R>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<R>>,
    param: StaticSystemParam<<R as MutRenderAsset>::Param>,
) {
    let mut param = param.into_inner();
    let mut queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, extracted_asset) in queued_assets.drain(..) {
        match R::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(handle, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((handle, extracted_asset));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_assets.remove(&removed);
    }

    for (handle, extracted_asset) in std::mem::take(&mut extracted_assets.extracted) {
        match R::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(handle, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((handle, extracted_asset));
            }
        }
    }
}
