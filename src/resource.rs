use bevy::ecs::system::{Resource, StaticSystemParam, SystemParam, SystemParamItem};
use bevy::prelude::*;
use bevy::render::render_asset::PrepareAssetLabel;
use bevy::render::{RenderApp, RenderStage};
use std::marker::PhantomData;
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
//TODO simplify, global unique asset extracting + preparing for renderapp
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub enum PrepareResourceError<E: Send + Sync + 'static> {
    RetryNextUpdate(E),
}

/// Describes how a resource gets extracted and prepared for rendering.
///
/// In the [`RenderStage::Extract`](crate::RenderStage::Extract) step the resource is transferred
/// from the "app world" into the "render world".
/// Therefore it is converted into a [`RenderResource::ExtractedResource`], which may be the same type
/// as the render resource itself.
///
/// After that in the [`RenderStage::Prepare`](crate::RenderStage::Prepare) step the extracted resource
/// is transformed into its GPU-representation of type [`RenderResource::PreparedResource`].
pub trait RenderResource: Resource {
    /// The representation of the the resource in the "render world".
    type ExtractedResource: Send + Sync + 'static;
    /// The GPU-representation of the the resource.
    type PreparedResource: Send + Sync + 'static;
    /// Specifies all ECS data required by [`RenderResource::prepare_resource`].
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) SystemParams.
    type Param: SystemParam;
    /// Converts the resource into a [`RenderResource::ExtractedResource`].
    fn extract_resource(&self) -> Self::ExtractedResource;
    /// Prepares the `extracted resource` for the GPU by transforming it into
    /// a [`RenderResource::PreparedResource`]. Therefore ECS data may be accessed via the `param`.
    fn prepare_resource(
        extracted_resource: Self::ExtractedResource,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>>;
}

/// This plugin extracts the changed resources from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`RenderResources`] resource.
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) and
/// [`RenderStage::Prepare`](crate::RenderStage::Prepare) steps for the specified [`RenderResource`].
pub struct RenderResourcePlugin<A: RenderResource>(PhantomData<fn() -> A>);

impl<A: RenderResource> Default for RenderResourcePlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: RenderResource> Plugin for RenderResourcePlugin<A> {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<UpdatedResource<A>>()
            .init_resource::<PreparedRenderResource<A>>()
            .init_resource::<PrepareNextFrameResource<A>>()
            .add_system_to_stage(RenderStage::Extract, extract_render_resource::<A>)
            .add_system_to_stage(
                RenderStage::Prepare,
                // this is important as renderresources may point to assets which
                // have to be available when resources are prepared (e.g. images)!
                prepare_resource::<A>
                    .after(PrepareAssetLabel::PreAssetPrepare)
                    .after("prepare_assets"),
            );
    }
}

/// Temporarily stores the extracted and removed resources of the current frame.
pub enum UpdatedResource<A: RenderResource> {
    Unchanged,
    Updated(Option<A::ExtractedResource>),
    Removed,
}

impl<A: RenderResource> Default for UpdatedResource<A> {
    fn default() -> Self {
        Self::Unchanged
    }
}

/// Stores all GPU representations ([`RenderResource::PreparedResources`](RenderResource::PreparedResource))
/// of [`RenderResources`](RenderResource) as long as they exist.
pub type PreparedRenderResource<A> = Option<<A as RenderResource>::PreparedResource>;

/// This system extracts all crated or modified resources of the corresponding [`RenderResource`] type
/// into the "render world".
fn extract_render_resource<A: RenderResource>(mut commands: Commands, resource: Option<Res<A>>) {
    match resource {
        Some(resource) => {
            if resource.is_changed() || resource.is_added() {
                commands.insert_resource::<UpdatedResource<A>>(UpdatedResource::Updated(Some(
                    resource.extract_resource(),
                )));
            } else {
                commands.insert_resource::<UpdatedResource<A>>(UpdatedResource::Unchanged);
            }
        }
        None => {
            // commands.remove_resource::<UpdatedResource<A>>();
            commands.insert_resource::<UpdatedResource<A>>(UpdatedResource::Removed);
        }
    }
}

// TODO: consider storing inside system?
/// All resources that should be prepared next frame.
pub struct PrepareNextFrameResource<A: RenderResource> {
    resource: Option<A::ExtractedResource>,
}

impl<A: RenderResource> Default for PrepareNextFrameResource<A> {
    fn default() -> Self {
        Self { resource: None }
    }
}

/// This system prepares a resource of the corresponding [`RenderResource`] type
/// which was extracted this frame for the GPU.
fn prepare_resource<R: RenderResource>(
    updated: ResMut<UpdatedResource<R>>,
    mut prepared: ResMut<PreparedRenderResource<R>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameResource<R>>,
    param: StaticSystemParam<<R as RenderResource>::Param>,
) {
    let mut param = param.into_inner();
    match updated.into_inner() {
        UpdatedResource::Unchanged => {
            if let Some(queued_resource) = prepare_next_frame.resource.take() {
                match R::prepare_resource(queued_resource, &mut param) {
                    Ok(prepared_resource) => {
                        *prepared.as_mut() = Some(prepared_resource);
                    }
                    Err(PrepareResourceError::RetryNextUpdate(extracted_resource)) => {
                        prepare_next_frame.resource = Some(extracted_resource);
                    }
                }
            }
        }
        UpdatedResource::Updated(extracted_resource) => {
            prepare_next_frame.resource = None;
            match R::prepare_resource(extracted_resource.take().unwrap(), &mut param) {
                Ok(prepared_resource) => {
                    // warn!("renderresource.Updated: setting prepared_resource");
                    *prepared.into_inner() = Some(prepared_resource);
                }
                Err(PrepareResourceError::RetryNextUpdate(extracted_resource)) => {
                    // warn!("renderresource.RetryNextUpdate");
                    prepare_next_frame.resource = Some(extracted_resource);
                }
            }
        }
        UpdatedResource::Removed => {
            prepare_next_frame.resource = None;
            prepared.into_inner().take();
        }
    }
}
