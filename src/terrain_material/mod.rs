// ----------------------------------------------------------------------------
#[cfg(debug_assertions)]
const TERRAIN_TEXTURE_MIP_LEVELS: Option<u8> = None;
#[cfg(not(debug_assertions))]
const TERRAIN_TEXTURE_MIP_LEVELS: Option<u8> = Some(0);
// ----------------------------------------------------------------------------
use bevy::{
    ecs::schedule::StateData,
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::TextureFormat,
    tasks::{IoTaskPool, Task},
};

use futures_lite::Future;

use crate::config::TerrainConfig;
use crate::loader::LoaderPlugin;
use crate::texturearray::{TextureArray, TextureArrayBuilder, TextureMipLevel};
use crate::DefaultResources;
use crate::{
    cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent},
    EditorEvent,
};
// ----------------------------------------------------------------------------
#[derive(Default, Clone, TypeUuid)]
#[uuid = "867a207f-7ada-4fdd-8319-df7d383fa6ff"]
/// handles to all diffuse and normal textures and parameter settings
pub struct TerrainMaterialSet {
    pub diffuse: Handle<TextureArray>,
    pub normal: Handle<TextureArray>,
    pub parameter: [TerrainMaterialParam; 31],
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum TextureType {
    Diffuse,
    Normal,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialSlot(u8);
// ----------------------------------------------------------------------------
pub struct TextureUpdatedEvent(pub MaterialSlot, pub TextureType);
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct TerrainMaterialParam {
    pub blend_sharpness: f32,
    pub slope_base_dampening: f32,
    pub slope_normal_dampening: f32,
    pub specularity_scale: f32,
    pub specularity: f32,
    pub specularity_base: f32,
    pub _specularity_scale_copy: f32,
    pub falloff: f32,
}
// ----------------------------------------------------------------------------
pub struct MaterialSetPlugin;
// ----------------------------------------------------------------------------
impl MaterialSetPlugin {
    // ------------------------------------------------------------------------
    /// normal active cam operation
    pub fn setup_default_materialset<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(setup_default_materialset)
    }
    // ------------------------------------------------------------------------
    pub fn terrain_material_loading<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state)
            .with_system(start_material_tasks)
            .with_system(check_material_tasks)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for MaterialSetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMaterialSet>()
            .init_resource::<MaterialLoadingTaskQueue>();
    }
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct MaterialLoadingTaskQueue {
    pending: usize,
}
// ----------------------------------------------------------------------------
impl MaterialLoadingTaskQueue {
    // ------------------------------------------------------------------------
    /// returns true if last pending was "finished"
    fn finished(&mut self, count: usize) -> bool {
        if self.pending > 0 {
            self.pending = self.pending.saturating_sub(count);
            self.pending == 0
        } else {
            false
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
struct TerrainTextureData {
    slot: MaterialSlot,
    ty: TextureType,
    mips: Vec<TextureMipLevel>,
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(super) fn setup_default_materialset(
    placeholder: Res<DefaultResources>,
    textures: Res<Assets<Image>>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    mut materialset: ResMut<TerrainMaterialSet>,
    mut editor_events: EventWriter<EditorEvent>,
) {
    use EditorEvent::TerrainTextureUpdated;
    use TextureFormat::Rgba8UnormSrgb;
    use TextureType::*;

    debug!("generating default material pallete.start");

    let logo = textures.get(&placeholder.logo).expect("loaded logo");

    // create placeholder texture arrays
    let dim = 1024;

    // let default_color = [128u8, 128u8, 128u8, 128u8];
    let default_color = [255u8, 255u8, 255u8, 255u8];
    let default_normal = [0u8, 0u8, 255u8, 0u8];

    let mut placeholder_diffuse = default_color.repeat(dim * dim);
    let placeholder_normal = default_normal.repeat(dim * dim);
    // add logo

    let logo_width_bytes = (logo.texture_descriptor.size.width * 4) as usize;
    for y in 0..logo.texture_descriptor.size.height as usize {
        let logo_offset = y * logo_width_bytes;
        let logo_slice = &logo.data[logo_offset..logo_offset + logo_width_bytes];

        let offset = 4 * (y * dim + 200);
        placeholder_diffuse[offset..offset + logo_width_bytes].copy_from_slice(logo_slice);
    }

    let dim = dim as u32;
    let placeholder_diffuse = image::DynamicImage::ImageRgba8(
        image::ImageBuffer::from_raw(dim, dim, placeholder_diffuse).unwrap(),
    );
    let placeholder_normal = image::DynamicImage::ImageRgba8(
        image::ImageBuffer::from_raw(dim, dim, placeholder_normal).unwrap(),
    );

    let mut diffuse_array =
        TextureArrayBuilder::new(dim, 31, Rgba8UnormSrgb, TERRAIN_TEXTURE_MIP_LEVELS);
    let mip_sizes = diffuse_array.mip_sizes();
    let placeholder_diffuse = TextureArray::generate_mips(placeholder_diffuse, &mip_sizes);
    let placeholder_normal = TextureArray::generate_mips(placeholder_normal, &mip_sizes);

    let mut update_events = Vec::with_capacity(2 * 31);

    for slot in 0..31 {
        diffuse_array.add_texture_with_mips(placeholder_diffuse.clone());

        update_events.push(TerrainTextureUpdated(TextureUpdatedEvent(
            slot.into(),
            Diffuse,
        )));
    }
    materialset.diffuse = texture_arrays.add(diffuse_array.build());

    let mut normal_array =
        TextureArrayBuilder::new(dim as u32, 31, Rgba8UnormSrgb, TERRAIN_TEXTURE_MIP_LEVELS);
    for slot in 0..31 {
        normal_array.add_texture_with_mips(placeholder_normal.clone());

        update_events.push(TerrainTextureUpdated(TextureUpdatedEvent(
            slot.into(),
            Normal,
        )));
    }
    materialset.normal = texture_arrays.add(normal_array.build());

    // notify editor to update preview images in ui
    editor_events.send_batch(update_events.drain(..));

    debug!("generating default material pallete.end");
}
// ----------------------------------------------------------------------------
fn load_terrain_texture(
    slot: MaterialSlot,
    filepath: String,
    texture_size: u32,
    texture_type: TextureType,
    mip_sizes: Vec<u32>,
) -> impl Future<Output = Result<TerrainTextureData, String>> {
    use image::DynamicImage::ImageRgba8;
    async move {
        let data = LoaderPlugin::load_terrain_texture(filepath, texture_size).await?;

        let mips = TextureArray::generate_mips(ImageRgba8(data), &mip_sizes);
        Ok(TerrainTextureData {
            slot,
            ty: texture_type,
            mips,
        })
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn start_material_tasks(
    mut commands: Commands,
    mut tasks_queued: EventReader<AsyncTaskStartEvent>,
    mut loading_queue: ResMut<MaterialLoadingTaskQueue>,
    mut materialset: ResMut<TerrainMaterialSet>,
    terrain_config: Res<TerrainConfig>,
    thread_pool: Res<IoTaskPool>,
) {
    for task in tasks_queued.iter() {
        use AsyncTaskStartEvent::*;

        #[allow(clippy::single_match)]
        match task {
            LoadTerrainMaterialSet => {
                let materialset_config = terrain_config.materialset();

                // update texture params
                materialset.parameter = [TerrainMaterialParam::default(); 31];
                for (slot, p) in materialset_config.parameters() {
                    materialset.parameter[slot] = *p;
                }

                // schedule loading all textures
                let texture_size = materialset_config.texture_size();
                let mip_sizes =
                    TextureArray::calculate_mip_sizes(texture_size, TERRAIN_TEXTURE_MIP_LEVELS);

                // spawn tasks for all materials to be loaded
                for (slot, diffuse, normal) in materialset_config.textures() {
                    let task = thread_pool.spawn(load_terrain_texture(
                        slot,
                        diffuse.to_string(),
                        texture_size,
                        TextureType::Diffuse,
                        mip_sizes.clone(),
                    ));
                    commands.spawn().insert(task);

                    let task = thread_pool.spawn(load_terrain_texture(
                        slot,
                        normal.to_string(),
                        texture_size,
                        TextureType::Normal,
                        mip_sizes.clone(),
                    ));
                    commands.spawn().insert(task);

                    loading_queue.pending += 2;
                }
            }
            _ => {
                // ignore all others
            }
        }
    }
}
// ----------------------------------------------------------------------------
type TaskResult<T> = Task<Result<T, String>>;

fn check_material_tasks(
    mut commands: Commands,
    mut loading_queue: ResMut<MaterialLoadingTaskQueue>,
    mut materialset: ResMut<TerrainMaterialSet>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    mut texture_tasks: Query<(Entity, &mut TaskResult<TerrainTextureData>)>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut editor_events: EventWriter<EditorEvent>,
) {
    use futures_lite::future;

    if !texture_tasks.is_empty() {
        // slightly faster to block only once and poll multiple tasks within the
        // future
        let task = async move {
            for (entity, mut task) in texture_tasks.iter_mut() {
                if let Some(new_texture) = future::poll_once(&mut *task).await {
                    match new_texture {
                        Ok(new_texture) => {
                            let handle = match new_texture.ty {
                                TextureType::Diffuse => &materialset.diffuse,
                                TextureType::Normal => &materialset.normal,
                            };
                            let array = texture_arrays
                                .get_mut(handle)
                                .expect("terrain texturematerial array image");

                            array.update_slot_with_mips(*new_texture.slot, new_texture.mips);

                            // notify editor to update preview images in ui
                            editor_events.send(EditorEvent::TerrainTextureUpdated(
                                TextureUpdatedEvent(new_texture.slot, new_texture.ty),
                            ));
                            debug!(
                                "{:?} texture ({}) loading finished",
                                new_texture.slot, new_texture.ty
                            );
                            materialset.set_changed();
                        }
                        Err(msg) => {
                            error!("failed to load material texture: {}", msg);
                            // notification_events.send(UserNotificationEvent::Error(msg.to_string())),
                        }
                    }
                    commands.entity(entity).despawn();

                    if loading_queue.finished(1) {
                        task_finished.send(AsyncTaskFinishedEvent::TerrainMaterialSetLoaded);
                    }
                }
            }
        };
        future::block_on(task);
    }
}
// ----------------------------------------------------------------------------
// material params
// ----------------------------------------------------------------------------
impl Default for TerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // TODO check defaults
        Self {
            blend_sharpness: 0.0,
            slope_base_dampening: 0.0,
            slope_normal_dampening: 0.5,
            specularity_scale: 0.0,
            specularity: 0.0,
            specularity_base: 0.0,
            _specularity_scale_copy: 0.0,
            falloff: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<u8> for MaterialSlot {
    fn from(v: u8) -> Self {
        MaterialSlot(v)
    }
}
// ----------------------------------------------------------------------------
impl std::ops::Deref for MaterialSlot {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
use std::ops::{Index, IndexMut};

impl Index<MaterialSlot> for [TerrainMaterialParam; 31] {
    type Output = TerrainMaterialParam;

    fn index(&self, index: MaterialSlot) -> &Self::Output {
        &self[*index as usize]
    }
}
// ----------------------------------------------------------------------------
impl IndexMut<MaterialSlot> for [TerrainMaterialParam; 31] {
    fn index_mut(&mut self, index: MaterialSlot) -> &mut Self::Output {
        &mut self[*index as usize]
    }
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// fmt
// ----------------------------------------------------------------------------
use std::fmt;

impl fmt::Display for TextureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureType::Diffuse => write!(f, "diffuse"),
            TextureType::Normal => write!(f, "normal"),
        }
    }
}
// ----------------------------------------------------------------------------
impl fmt::Display for MaterialSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
// ----------------------------------------------------------------------------
